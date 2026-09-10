mod idl;
// buffa emits view re-exports for every message; most modules use only the owned type.
#[allow(unused_imports)]
mod pb;

use pb::sf::solana::idl::v1::{DataSource, IdlAccountWrite, IdlAccountWrites, Source};
use pb::sf::solana::r#type::v1::{Account, AccountBlock};
use substreams::errors::Error;
use substreams::pb::sf::substreams::index::v1::Keys;

/// Single index key. The package indexes one thing — "this block touched an IDL
/// account" — so a constant beats a parameterised query: the deployed package stays
/// self-contained and every deployment shares one index.
const IDL_KEY: &str = "idl";

/// Marks blocks containing at least one IDL write so the map below can skip the
/// rest. IDL writes are rare, so this is what makes a chain-wide scan affordable.
#[substreams::handlers::map]
fn index_idl_accounts(block: AccountBlock) -> Result<Keys, Error> {
    Ok(index_keys(&block))
}

// The handler attribute rewrites the function it decorates into a wasm export, so
// the bodies live here where tests can reach them.
fn index_keys(block: &AccountBlock) -> Keys {
    let hit = block.accounts.iter().any(|a| idl::is_idl_account(&a.owner, &a.data));

    Keys {
        keys: if hit { vec![IDL_KEY.to_string()] } else { vec![] },
    }
}

/// Decodes every IDL write in a block.
///
/// Reads the raw account stream rather than the `filtered_accounts` foundational
/// module: that module only indexes `account:` and `owner:` with exact matching, and
/// legacy IDL accounts are owned by the program they describe, so no owner filter can
/// enumerate them. Taking the source directly also keeps slot and block time, which
/// `FilteredAccounts` drops.
#[substreams::handlers::map]
fn map_idl_accounts(block: AccountBlock) -> Result<IdlAccountWrites, Error> {
    Ok(collect_writes(&block))
}

fn collect_writes(block: &AccountBlock) -> IdlAccountWrites {
    let slot = block.slot;
    let block_time = block.timestamp.seconds;

    // Per-block allowance, not per-account: without it a block carrying a dozen
    // bombs multiplies the per-payload ceiling by a dozen, which on wasm32 is
    // enough to exhaust the address space and poison the block permanently.
    let mut allowance = MAX_BLOCK_DECOMPRESSED_LEN;
    let writes = block
        .accounts
        .iter()
        .filter(|a| idl::is_idl_account(&a.owner, &a.data))
        .map(|a| clamp_varchars(decode_write(a, slot, block_time, &mut allowance)))
        .collect();

    IdlAccountWrites { writes }
}

/// What every IDL payload in one block may decompress to, in total. Real blocks hold
/// one or two IDL writes of ~110 KB, so this is far above anything legitimate.
const MAX_BLOCK_DECOMPRESSED_LEN: usize = 8 * 1024 * 1024;

/// `substreams-sink-sql from-proto` maps every proto `string` to `VARCHAR(255)`.
/// Several of these strings come from account data nobody authenticates, and there
/// are two ways such a string fails a Postgres insert outright: it is longer than
/// the column, or it contains a NUL, which no Postgres text type can store. Either
/// error stalls the sink for good, because the offending block replays on every
/// backfill. `idl_json` sidesteps the column limit by being `bytes`, which maps to
/// unbounded `BYTEA`; every `string` field has to come through here instead.
const VARCHAR_LEN: usize = 255;

fn clamp_varchars(mut w: IdlAccountWrite) -> IdlAccountWrite {
    let mut truncated = false;
    for field in [
        &mut w.id,
        &mut w.account_address,
        &mut w.program_id,
        &mut w.authority,
        &mut w.seed,
        &mut w.version,
        &mut w.external_url,
        &mut w.external_account,
        &mut w.decode_error,
    ] {
        // An interior NUL survives both from_utf8_lossy over raw account bytes and a
        // \u0000 escape in the IDL's own JSON, so trimming the ends is not enough.
        if field.contains('\0') {
            *field = field.replace('\0', "");
        }
        // Postgres counts characters, not bytes, and truncating mid-codepoint would
        // panic, so cut on a char boundary.
        if field.chars().count() > VARCHAR_LEN {
            *field = field.chars().take(VARCHAR_LEN).collect();
            truncated = true;
        }
    }
    // A cut URL still looks like a URL, so say so rather than let a consumer fetch it.
    if truncated && w.decode_error.is_empty() {
        w.decode_error = format!("a field was truncated to {VARCHAR_LEN} characters");
    }
    w
}

fn decode_write(account: &Account, slot: u64, block_time: i64, allowance: &mut usize) -> IdlAccountWrite {
    let address = bs58::encode(&account.address).into_string();

    let base = IdlAccountWrite {
        id: format!("{address}:{slot}"),
        account_address: address,
        slot,
        block_time,
        deleted: account.deleted,
        version_major: -1,
        ..Default::default()
    };

    if account.owner == idl::METADATA_PROGRAM_ID {
        return decode_metadata(account, base, allowance);
    }
    decode_legacy(account, base, allowance)
}

fn decode_legacy(account: &Account, mut write: IdlAccountWrite, allowance: &mut usize) -> IdlAccountWrite {
    // create_with_seed(base, "anchor:idl", program_id) takes the program as the owner,
    // so the account owner is the program the IDL describes.
    write.program_id = bs58::encode(&account.owner).into_string();
    write.source = Source::Legacy.into();
    write.compression = 2; // legacy IDLs are always zlib

    let parsed = match idl::parse_legacy(&account.data) {
        Ok(parsed) => parsed,
        Err(e) => {
            write.decode_error = format!("legacy header: {e}");
            write.payload_len = account.data.len().saturating_sub(44) as u32;
            write.payload_omitted = write.payload_len > 0;
            return write;
        }
    };

    // Matches the metadata path: an all-zero authority means none is set, and the
    // column is documented as empty in that case rather than holding base58 zeroes.
    write.authority = if parsed.authority.iter().all(|b| *b == 0) {
        String::new()
    } else {
        bs58::encode(&parsed.authority).into_string()
    };
    finish(write, parsed.payload, 2, allowance)
}

fn decode_metadata(account: &Account, mut write: IdlAccountWrite, allowance: &mut usize) -> IdlAccountWrite {
    write.source = Source::ProgramMetadata.into();

    let m = match idl::parse_metadata(&account.data) {
        Ok(m) => m,
        Err(e) => {
            write.decode_error = format!("metadata header: {e}");
            write.payload_len = account.data.len().saturating_sub(96) as u32;
            write.payload_omitted = write.payload_len > 0;
            return write;
        }
    };

    write.program_id = bs58::encode(&m.program).into_string();
    write.authority = if m.authority.is_empty() {
        String::new()
    } else {
        bs58::encode(&m.authority).into_string()
    };
    write.canonical = m.canonical;
    write.seed = m.seed;
    write.encoding = m.encoding;
    write.compression = m.compression;
    write.format = m.format;
    write.data_source = (m.data_source as i32).into();

    // URL and External hold a pointer, not the document, so there is nothing to
    // decompress. Record the pointer and leave idl_json empty.
    write.payload_len = m.payload.len() as u32;
    if m.data_source == DataSource::Url as u32 {
        write.external_url = String::from_utf8_lossy(&m.payload).trim_end_matches('\0').to_string();
        return write;
    }
    if m.data_source == DataSource::External as u32 {
        if m.payload.len() >= 32 {
            write.external_account = bs58::encode(&m.payload[..32]).into_string();
        } else {
            write.decode_error = format!(
                "external data source needs a 32-byte account, payload holds {}",
                m.payload.len()
            );
            write.payload_omitted = !m.payload.is_empty();
        }
        return write;
    }

    let compression = m.compression;
    finish(write, m.payload, compression, allowance)
}

fn finish(mut write: IdlAccountWrite, payload: Vec<u8>, compression: u32, allowance: &mut usize) -> IdlAccountWrite {
    write.payload_len = payload.len() as u32;

    if write.deleted {
        // Bytes may still be present in the write that deleted the account; they are
        // deliberately not stored, which is what payload_omitted is for.
        write.payload_omitted = !payload.is_empty();
        return write;
    }
    if payload.is_empty() {
        write.empty_payload = true;
        return write;
    }

    let limit = (*allowance).min(idl::MAX_DECOMPRESSED_LEN);
    let decompressed = idl::decompress(&payload, compression, limit);
    if let Ok(json) = &decompressed {
        // Charged whether or not the document turns out to be an IDL — the memory was
        // spent inflating it either way, which is what the allowance is rationing.
        *allowance = allowance.saturating_sub(json.len());
    }
    match decompressed {
        Ok(json) => match idl::parse_complete(&json) {
            Ok(parsed) => {
                let (version, major) = idl::extract_version(&parsed);
                write.version = version;
                write.version_major = major;
                write.idl_json = json.into_bytes();
                write.complete = true;
                // Keep the bytes only here: a completed document supersedes every
                // chunk that built it, and those chunks are prefixes of it.
                write.payload_raw = payload;
            }
            // A chunk from an in-flight upload. Recording its half-document as a
            // version would invent IDLs that never existed on chain.
            Err(e) => {
                write.decode_error = e;
                write.payload_omitted = true;
            }
        },
        Err(e) => {
            write.decode_error = e;
            write.payload_omitted = true;
        }
    }

    write
}

#[cfg(test)]
mod tests {
    use super::*;
    use pb::sf::solana::idl::v1::Source;

    const METADATA_HEADER_LEN: usize = 96;

    /// Every test goes through a fresh block allowance, as the map handler does.
    fn decoded(a: &Account) -> IdlAccountWrite {
        let mut allowance = MAX_BLOCK_DECOMPRESSED_LEN;
        decode_write(a, 1, 2, &mut allowance)
    }

    fn account(owner: Vec<u8>, data: Vec<u8>) -> Account {
        Account {
            address: vec![7u8; 32],
            owner,
            data,
            ..Default::default()
        }
    }

    fn legacy_account(payload: &[u8]) -> Account {
        let mut data = Vec::new();
        data.extend_from_slice(&idl::LEGACY_DISCRIMINATOR);
        data.extend_from_slice(&[3u8; 32]);
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
        account(vec![9u8; 32], data)
    }

    fn metadata_account(data_source: u8, payload: &[u8]) -> Account {
        metadata_account_with(data_source, 0, payload)
    }

    fn metadata_account_with(data_source: u8, compression: u8, payload: &[u8]) -> Account {
        let mut data = vec![0u8; METADATA_HEADER_LEN];
        data[0] = idl::METADATA_DISCRIMINATOR_METADATA;
        data[66] = 1; // canonical
        data[67..70].copy_from_slice(b"idl");
        data[84] = compression;
        data[86] = data_source;
        data[87..91].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
        account(idl::METADATA_PROGRAM_ID.to_vec(), data)
    }

    fn zlib(text: &str) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(text.as_bytes()).unwrap();
        enc.finish().unwrap()
    }

    fn block_of(accounts: Vec<Account>) -> AccountBlock {
        AccountBlock {
            slot: 1,
            accounts,
            ..Default::default()
        }
    }

    /// Goes through the handler rather than calling the pieces, because the clamp is
    /// wired in there. Asserting it via a direct clamp_varchars call lets the wiring
    /// be deleted with every test still green.
    #[test]
    fn the_handler_clamps_what_it_emits() {
        let url = "h".repeat(4000);
        let out = collect_writes(&block_of(vec![metadata_account(1, url.as_bytes())]));
        assert_eq!(out.writes.len(), 1);
        assert_eq!(out.writes[0].external_url.chars().count(), VARCHAR_LEN);
    }

    /// One block, many bombs: the allowance is spent once, not once per account.
    #[test]
    fn the_handler_bounds_a_whole_block_not_each_account() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let big = format!(r#"{{"instructions":[],"pad":"{}"}}"#, "a".repeat(900 * 1024));
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
        enc.write_all(big.as_bytes()).unwrap();
        let bomb = enc.finish().unwrap();

        let accounts: Vec<Account> = (0..32).map(|_| legacy_account(&bomb)).collect();
        let out = collect_writes(&block_of(accounts));

        let stored: usize = out.writes.iter().map(|w| w.idl_json.len()).sum();
        assert!(stored > 0, "fixture must actually decode, or the bound is untested");
        assert!(
            stored <= MAX_BLOCK_DECOMPRESSED_LEN,
            "block retained {stored} bytes, over the {MAX_BLOCK_DECOMPRESSED_LEN} allowance"
        );
        assert!(
            out.writes.iter().any(|w| w.decode_error.contains("allowance")),
            "later accounts should be refused once the block allowance is spent"
        );
    }

    #[test]
    fn the_index_agrees_with_the_map_on_what_counts() {
        let hit = block_of(vec![legacy_account(&zlib("{}"))]);
        assert_eq!(index_keys(&hit).keys, vec![IDL_KEY.to_string()]);

        let miss = block_of(vec![account(vec![9u8; 32], vec![0u8; 200])]);
        assert!(index_keys(&miss).keys.is_empty());
    }

    /// The newer of the two formats, decoded end to end. Nothing else in the suite
    /// takes a Program Metadata account all the way to `complete`.
    #[test]
    fn metadata_account_decodes_to_a_complete_version() {
        let idl = r#"{"metadata":{"version":"3.1.4"},"instructions":[]}"#;
        let w = decoded(&metadata_account_with(0, 2, &zlib(idl)));

        assert_eq!(w.source, Source::ProgramMetadata as i32);
        assert!(w.complete, "{}", w.decode_error);
        assert_eq!(w.version, "3.1.4");
        assert_eq!(w.version_major, 3);
        assert_eq!(w.seed, "idl");
        assert!(w.canonical);
        assert_eq!(w.compression, 2);
        // Named in the header rather than inferred from the owner, unlike legacy.
        assert_eq!(w.program_id, bs58::encode([0u8; 32]).into_string());
    }

    /// The header's compression byte has to drive the decode; hard-coding zlib would
    /// silently work for legacy and corrupt every gzip-compressed metadata account.
    #[test]
    fn metadata_honours_the_header_compression_byte() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let idl = r#"{"metadata":{"version":"1.0.0"},"instructions":[]}"#;
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(idl.as_bytes()).unwrap();
        let gz = enc.finish().unwrap();

        let w = decoded(&metadata_account_with(0, 1, &gz));
        assert!(w.complete, "{}", w.decode_error);
        assert_eq!(w.version, "1.0.0");
    }

    #[test]
    fn a_delete_never_stores_a_document() {
        let idl = r#"{"metadata":{"version":"9.9.9"},"instructions":[]}"#;
        let mut a = legacy_account(&zlib(idl));
        a.deleted = true;

        let w = decoded(&a);
        assert!(w.deleted);
        assert!(!w.complete, "a delete must not be recorded as a version");
        assert!(w.idl_json.is_empty());
        assert!(w.payload_raw.is_empty());
        assert!(w.payload_omitted, "bytes were present and deliberately dropped");
    }

    /// These are the sink's output contract, not internals: `id` is the primary key,
    /// and the rest are columns consumers filter on.
    #[test]
    fn the_row_carries_its_identity_and_provenance() {
        let mut allowance = MAX_BLOCK_DECOMPRESSED_LEN;
        let a = legacy_account(&zlib(r#"{"instructions":[]}"#));
        let w = decode_write(&a, 4242, 1_700_000_000, &mut allowance);

        let address = bs58::encode(&a.address).into_string();
        assert_eq!(w.id, format!("{address}:4242"), "primary key is address:slot");
        assert_eq!(w.account_address, address);
        assert_eq!(w.slot, 4242);
        assert_eq!(w.block_time, 1_700_000_000);
        assert_eq!(w.compression, 2, "legacy IDLs are always zlib");
        assert_eq!(w.authority, bs58::encode([3u8; 32]).into_string());
    }

    #[test]
    fn a_legacy_header_error_still_reports_the_bytes_it_saw() {
        let mut data = idl::LEGACY_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&[3u8; 32]);
        data.extend_from_slice(&999_999u32.to_le_bytes());
        data.extend_from_slice(&[7u8; 900]);

        let w = decoded(&account(vec![9u8; 32], data));
        assert!(
            w.decode_error.contains("declares a 999999-byte payload"),
            "{}",
            w.decode_error
        );
        assert_eq!(w.payload_len, 900);
        assert!(w.payload_omitted);
    }

    #[test]
    fn truncating_a_field_is_recorded_not_silent() {
        let url = "h".repeat(4000);
        let w = clamp_varchars(decoded(&metadata_account(1, url.as_bytes())));
        assert_eq!(w.external_url.chars().count(), VARCHAR_LEN);
        assert!(w.decode_error.contains("truncated"), "a cut URL still looks like a URL");
    }

    #[test]
    fn routes_on_owner_not_content() {
        let legacy = decoded(&legacy_account(&zlib("{}")));
        assert_eq!(legacy.source, Source::Legacy as i32);
        // create_with_seed makes the described program the owner, so it is the id.
        assert_eq!(legacy.program_id, bs58::encode([9u8; 32]).into_string());

        let meta = decoded(&metadata_account(0, &zlib("{}")));
        assert_eq!(meta.source, Source::ProgramMetadata as i32);

        // The discriminating case: legacy content under the metadata program. Owner
        // decides, so this is a malformed metadata write, not a legacy one.
        let mut data = idl::LEGACY_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&[0u8; 40]);
        let confused = decoded(&account(idl::METADATA_PROGRAM_ID.to_vec(), data));
        assert_eq!(confused.source, Source::ProgramMetadata as i32);
        assert!(
            confused.decode_error.starts_with("metadata header:"),
            "{}",
            confused.decode_error
        );
    }

    #[test]
    fn oversized_strings_are_clamped_to_what_varchar_holds() {
        let url = "h".repeat(4000);
        let w = clamp_varchars(decoded(&metadata_account(1, url.as_bytes())));
        assert_eq!(w.external_url.chars().count(), VARCHAR_LEN);

        let idl = format!(r#"{{"version":"{}"}}"#, "9".repeat(4000));
        let w = clamp_varchars(decoded(&legacy_account(&zlib(&idl))));
        assert_eq!(w.version.chars().count(), VARCHAR_LEN);
    }

    #[test]
    fn interior_nul_is_stripped_because_postgres_text_cannot_hold_it() {
        // Valid JSON: serde decodes the escape into a real NUL inside the version.
        let idl = "{\"version\":\"1.0\\u00000\"}";
        let w = clamp_varchars(decoded(&legacy_account(&zlib(idl))));
        assert!(w.complete);
        assert!(!w.version.contains('\0'), "{:?}", w.version);

        let w = clamp_varchars(decoded(&metadata_account(1, b"http://a\0b")));
        assert!(!w.external_url.contains('\0'), "{:?}", w.external_url);
    }

    #[test]
    fn short_external_payload_reports_why_rather_than_going_blank() {
        let w = decoded(&metadata_account(2, &[1u8; 8]));
        assert!(w.external_account.is_empty());
        assert!(w.decode_error.contains("32-byte account"), "{}", w.decode_error);
    }

    #[test]
    fn json_that_is_not_an_object_is_not_a_version() {
        let w = decoded(&legacy_account(&zlib("123")));
        assert!(!w.complete);
        assert!(w.decode_error.contains("not an IDL object"), "{}", w.decode_error);
        assert!(w.payload_omitted);
    }

    #[test]
    fn complete_document_keeps_payload_and_version() {
        let idl = r#"{"metadata":{"version":"2.3.4"},"instructions":[]}"#;
        let w = decoded(&legacy_account(&zlib(idl)));
        assert!(w.complete);
        assert_eq!(w.version, "2.3.4");
        assert_eq!(w.version_major, 2);
        assert!(!w.payload_raw.is_empty());
        assert!(w.decode_error.is_empty());
    }

    #[test]
    fn upload_chunk_is_recorded_without_its_half_document() {
        let chunk = &zlib(r#"{"metadata":{"version":"1.0.0"},"instru"#);
        let w = decoded(&legacy_account(chunk));
        assert!(!w.complete);
        assert!(w.payload_omitted);
        assert!(w.idl_json.is_empty());
        assert!(w.payload_raw.is_empty());
    }

    #[test]
    fn empty_payload_is_distinguished_from_a_decode_failure() {
        // Allocated and over-allocated but never written: the header declares a
        // zero-length payload while the account still holds slack bytes.
        let mut data = Vec::new();
        data.extend_from_slice(&idl::LEGACY_DISCRIMINATOR);
        data.extend_from_slice(&[3u8; 32]);
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 64]);

        let w = decoded(&account(vec![9u8; 32], data));
        assert!(w.empty_payload);
        assert!(w.decode_error.is_empty());
        assert!(!w.payload_omitted);
    }
}
