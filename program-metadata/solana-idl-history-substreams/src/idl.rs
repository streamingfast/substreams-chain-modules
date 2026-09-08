//! Recognising and decoding IDL-bearing Solana accounts.
//!
//! Two unrelated on-chain formats carry Anchor IDLs:
//!
//! * legacy — an Anchor account created at
//!   `create_with_seed(find_program_address([], program), "anchor:idl", program)`.
//!   The third argument to `create_with_seed` is the owner, so the account is owned
//!   by the program it describes and attribution needs no lookup.
//! * SPL Program Metadata — a 96-byte header that names its program explicitly.
//!   Anchor integrated it in 1.0.0; adoption is still small next to legacy.

use flate2::read::{GzDecoder, ZlibDecoder};
use std::io::Read;

/// `sha256("internal:IdlAccount")[..8]`, the Anchor discriminator for `IdlAccount`.
pub const LEGACY_DISCRIMINATOR: [u8; 8] = [0x18, 0x46, 0x62, 0xbf, 0x3a, 0x90, 0x7b, 0x9e];

/// SPL Program Metadata program, base58 `ProgM6JCCvbYkfKqJYHePx4xxSUSqJp7rh8Lyv7nk7S`.
pub const METADATA_PROGRAM_ID: [u8; 32] = [
    5, 219, 23, 158, 195, 62, 147, 249, 75, 195, 33, 87, 7, 228, 251, 4, 116, 62, 201, 175, 168, 74, 27, 160, 111, 104,
    118, 139, 246, 101, 56, 137,
];

/// Header discriminator for a populated Program Metadata account. `0` is an empty
/// account and `1` is an in-progress upload buffer; only `2` is finished metadata.
pub const METADATA_DISCRIMINATOR_METADATA: u8 = 2;

const LEGACY_HEADER_LEN: usize = 8 + 32 + 4;
const METADATA_HEADER_LEN: usize = 96;

/// Cheap test used by both the block index and the map, so the two cannot drift.
/// Deliberately does no decoding — a block index only decides whether a block is
/// worth reading at all.
pub fn is_idl_account(owner: &[u8], data: &[u8]) -> bool {
    if data.len() > LEGACY_HEADER_LEN && data[..8] == LEGACY_DISCRIMINATOR {
        return true;
    }
    owner == METADATA_PROGRAM_ID
        && data.len() >= METADATA_HEADER_LEN
        && data[0] == METADATA_DISCRIMINATOR_METADATA
        && read_seed(data) == IDL_SEED
}

/// Program Metadata is a general-purpose store keyed by seed — `security` is another
/// standard one, and a program may invent its own. Only `idl` holds an IDL, and
/// without this check a `security` blob carrying a `version` field would be recorded
/// as a program version that never existed.
pub const IDL_SEED: &str = "idl";

/// Header bytes 67..83, NUL-padded to 16. Callers must have checked the length.
pub fn read_seed(data: &[u8]) -> String {
    String::from_utf8_lossy(&data[67..83])
        .trim_end_matches('\0')
        .to_string()
}

#[derive(Debug)]
pub struct LegacyIdl {
    pub authority: Vec<u8>,
    pub payload: Vec<u8>,
}

/// Layout: `[0..8]` discriminator, `[8..40]` authority, `[40..44]` u32 LE payload
/// length, `[44..]` payload. Accounts are over-allocated, so the length prefix —
/// not the account size — bounds the payload.
pub fn parse_legacy(data: &[u8]) -> Result<LegacyIdl, String> {
    if data.len() <= LEGACY_HEADER_LEN {
        return Err(format!(
            "account holds {} bytes, less than the {LEGACY_HEADER_LEN}-byte header",
            data.len()
        ));
    }
    if data[..8] != LEGACY_DISCRIMINATOR {
        return Err("not an IdlAccount discriminator".to_string());
    }
    let len = u32::from_le_bytes(data[40..44].try_into().expect("40..44 is four bytes")) as usize;
    let end = LEGACY_HEADER_LEN
        .checked_add(len)
        .ok_or_else(|| format!("declared payload length {len} overflows"))?;
    if end > data.len() {
        return Err(format!(
            "header declares a {len}-byte payload but the account holds {}",
            data.len() - LEGACY_HEADER_LEN
        ));
    }
    Ok(LegacyIdl {
        authority: data[8..40].to_vec(),
        payload: data[LEGACY_HEADER_LEN..end].to_vec(),
    })
}

#[derive(Debug, Default)]
pub struct MetadataIdl {
    pub program: Vec<u8>,
    pub authority: Vec<u8>,
    pub canonical: bool,
    pub seed: String,
    pub encoding: u32,
    pub compression: u32,
    pub format: u32,
    pub data_source: u32,
    pub payload: Vec<u8>,
}

/// Header layout from `solana-program/program-metadata` (`Header::LEN == 96`, align 1):
/// 0 discriminator, 1..33 program, 33..65 authority, 65 mutable, 66 canonical,
/// 67..83 seed, 83 encoding, 84 compression, 85 format, 86 data_source,
/// 87..91 data_length u32 LE, 91..96 padding.
pub fn parse_metadata(data: &[u8]) -> Result<MetadataIdl, String> {
    if data.len() < METADATA_HEADER_LEN {
        return Err(format!(
            "account holds {} bytes, less than the {METADATA_HEADER_LEN}-byte header",
            data.len()
        ));
    }
    if data[0] != METADATA_DISCRIMINATOR_METADATA {
        return Err("not a Metadata discriminator".to_string());
    }
    let authority = data[33..65].to_vec();
    let len = u32::from_le_bytes(data[87..91].try_into().expect("87..91 is four bytes")) as usize;
    let end = METADATA_HEADER_LEN
        .checked_add(len)
        .ok_or_else(|| format!("declared payload length {len} overflows"))?;
    if end > data.len() {
        return Err(format!(
            "header declares a {len}-byte payload but the account holds {}",
            data.len() - METADATA_HEADER_LEN
        ));
    }

    Ok(MetadataIdl {
        program: data[1..33].to_vec(),
        // An all-zero authority is the ZeroableOption "none".
        authority: if authority.iter().all(|b| *b == 0) {
            Vec::new()
        } else {
            authority
        },
        canonical: data[66] != 0,
        seed: read_seed(data),
        encoding: data[83] as u32,
        compression: data[84] as u32,
        format: data[85] as u32,
        data_source: data[86] as u32,
        payload: data[METADATA_HEADER_LEN..end].to_vec(),
    })
}

/// Ceiling on a single decompressed payload.
///
/// Nothing authenticates an IDL account — the legacy discriminator is eight bytes
/// anyone can write — so a crafted account can carry a compression bomb, and this
/// index has no program filter to hide behind. Without a cap the allocator aborts,
/// the module fails, and because the same block is replayed on every backfill the
/// failure is permanent.
///
/// The bound has to sit well under the memory the payload costs, not at it: the text
/// is parsed into a `serde_json::Value` tree afterwards, which for a scalar-dense
/// document runs to roughly 20x its length. The largest real IDL observed on chain
/// is ~110 KB, so 1 MiB keeps an order of magnitude of headroom while holding the
/// worst case per account to tens of megabytes rather than hundreds.
pub const MAX_DECOMPRESSED_LEN: usize = 1024 * 1024;

/// `compression`: 0 none, 1 gzip, 2 zlib. Legacy IDLs are always zlib.
///
/// `allowance` is what the caller has left to spend. Each call inflates at most
/// `allowance + 1` bytes before giving up, and only payloads that decode are charged
/// against it — so what a whole block is guaranteed to bound is the bytes it retains,
/// not the transient cost of refusing many bombs in a row.
pub fn decompress(payload: &[u8], compression: u32, allowance: usize) -> Result<String, String> {
    if payload.is_empty() {
        return Err("empty payload".to_string());
    }
    // One byte past the allowance so a payload landing exactly on it is still caught.
    let budget = (allowance as u64).saturating_add(1);
    let mut out = Vec::new();
    let read = match compression {
        0 => {
            out = payload.to_vec();
            Ok(0)
        }
        1 => GzDecoder::new(payload).take(budget).read_to_end(&mut out),
        2 => ZlibDecoder::new(payload).take(budget).read_to_end(&mut out),
        other => return Err(format!("unsupported compression {other}")),
    };
    read.map_err(|e| format!("decompress: {e}"))?;
    if out.len() > allowance {
        // Reporting what was read is what makes the streaming bound observable: with
        // the reader capped this is always allowance + 1, however large the bomb.
        return Err(format!(
            "decompressed payload exceeds the {allowance}-byte allowance (read {})",
            out.len()
        ));
    }
    String::from_utf8(out).map_err(|e| format!("utf8: {e}"))
}

/// Anchor uploads an IDL in chunks, so a mid-upload account holds a prefix of the
/// document. Those prefixes decompress cleanly and still contain a `"version"` field,
/// which makes them indistinguishable from a finished IDL by string inspection alone.
/// Parsing is the only honest completeness test.
pub fn parse_complete(idl_json: &str) -> Result<serde_json::Value, String> {
    let value: serde_json::Value =
        serde_json::from_str(idl_json).map_err(|e| format!("incomplete or invalid IDL JSON: {e}"))?;
    // Well-formed JSON is not the same as an IDL. `123` and `[]` parse, and marking
    // them complete would let anyone mint a "version" for a program they do not own.
    if !value.is_object() {
        return Err("payload is valid JSON but not an IDL object".to_string());
    }
    Ok(value)
}

/// Pull the semver from a parsed IDL. Anchor moved the field from top-level `version`
/// to `metadata.version` in 0.30 and both shapes are live on chain, so accept either.
/// Returns `(version, major)` with major `-1` when unknown — `0` is a legitimate
/// major and must stay distinguishable from "we don't know".
pub fn extract_version(idl: &serde_json::Value) -> (String, i64) {
    let version = idl
        .get("version")
        .and_then(|v| v.as_str())
        .or_else(|| idl.get("metadata")?.get("version")?.as_str())
        .unwrap_or_default()
        .to_string();

    let major = version
        .split('.')
        .next()
        .and_then(|m| m.trim().parse::<i64>().ok())
        .unwrap_or(-1);
    (version, major)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recomputes the hash rather than restating the bytes: a constant compared to a
    /// copy of itself cannot fail for any value of it.
    #[test]
    fn legacy_discriminator_is_the_anchor_hash() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"internal:IdlAccount");
        assert_eq!(LEGACY_DISCRIMINATOR, hash[..8]);
    }

    #[test]
    fn detects_legacy_account() {
        let mut data = vec![0u8; LEGACY_HEADER_LEN + 10];
        data[..8].copy_from_slice(&LEGACY_DISCRIMINATOR);
        assert!(is_idl_account(&[9u8; 32], &data));
    }

    #[test]
    fn ignores_unrelated_account() {
        assert!(!is_idl_account(&[9u8; 32], &[7u8; 200]));
    }

    #[test]
    fn ignores_legacy_header_with_no_payload() {
        let mut data = vec![0u8; LEGACY_HEADER_LEN];
        data[..8].copy_from_slice(&LEGACY_DISCRIMINATOR);
        assert!(!is_idl_account(&[9u8; 32], &data));
    }

    #[test]
    fn detects_metadata_account() {
        let mut data = vec![0u8; METADATA_HEADER_LEN];
        data[0] = METADATA_DISCRIMINATOR_METADATA;
        data[67..70].copy_from_slice(b"idl");
        assert!(is_idl_account(&METADATA_PROGRAM_ID, &data));
        // Same bytes under a different owner is not a metadata account.
        assert!(!is_idl_account(&[1u8; 32], &data));
    }

    #[test]
    fn metadata_buffer_discriminator_is_not_metadata() {
        let mut data = vec![0u8; METADATA_HEADER_LEN];
        data[67..70].copy_from_slice(b"idl");
        data[0] = 1; // Buffer
        assert!(!is_idl_account(&METADATA_PROGRAM_ID, &data));
    }

    #[test]
    fn parse_legacy_respects_length_prefix_not_account_size() {
        let mut data = vec![0u8; LEGACY_HEADER_LEN + 100];
        data[..8].copy_from_slice(&LEGACY_DISCRIMINATOR);
        data[8..40].copy_from_slice(&[3u8; 32]);
        data[40..44].copy_from_slice(&7u32.to_le_bytes());
        let parsed = parse_legacy(&data).expect("should parse");
        assert_eq!(parsed.authority, vec![3u8; 32]);
        assert_eq!(parsed.payload.len(), 7, "over-allocated tail must be ignored");
    }

    #[test]
    fn parse_legacy_rejects_length_past_end() {
        let mut data = vec![0u8; LEGACY_HEADER_LEN + 4];
        data[..8].copy_from_slice(&LEGACY_DISCRIMINATOR);
        data[40..44].copy_from_slice(&9999u32.to_le_bytes());
        let err = parse_legacy(&data).expect_err("should reject");
        assert!(err.contains("declares a"), "{err}");
    }

    #[test]
    fn parse_metadata_reads_header() {
        let mut data = vec![0u8; METADATA_HEADER_LEN + 4];
        data[0] = METADATA_DISCRIMINATOR_METADATA;
        data[1..33].copy_from_slice(&[8u8; 32]);
        data[33..65].copy_from_slice(&[4u8; 32]);
        data[66] = 1; // canonical
        data[67..70].copy_from_slice(b"idl");
        // Distinct values so transposing any two offsets fails.
        data[83] = 1; // encoding
        data[84] = 2; // compression, zlib
        data[85] = 3; // format
        data[86] = 1; // data_source, url
        data[87..91].copy_from_slice(&4u32.to_le_bytes());
        let m = parse_metadata(&data).expect("should parse");
        assert_eq!(m.program, vec![8u8; 32]);
        assert_eq!(m.authority, vec![4u8; 32]);
        assert!(m.canonical);
        assert_eq!(m.seed, "idl");
        assert_eq!(m.encoding, 1);
        assert_eq!(m.compression, 2);
        assert_eq!(m.format, 3);
        assert_eq!(m.data_source, 1);
        assert_eq!(m.payload.len(), 4);
    }

    #[test]
    fn metadata_accounts_under_another_seed_are_not_idls() {
        let mut data = vec![0u8; METADATA_HEADER_LEN + 4];
        data[0] = METADATA_DISCRIMINATOR_METADATA;
        data[67..75].copy_from_slice(b"security");
        assert!(
            !is_idl_account(&METADATA_PROGRAM_ID, &data),
            "a security blob is not a program's IDL"
        );

        data[67..83].fill(0);
        data[67..70].copy_from_slice(b"idl");
        assert!(is_idl_account(&METADATA_PROGRAM_ID, &data));
    }

    #[test]
    fn parse_metadata_treats_zero_authority_as_none() {
        let mut data = vec![0u8; METADATA_HEADER_LEN];
        data[0] = METADATA_DISCRIMINATOR_METADATA;
        let m = parse_metadata(&data).expect("should parse");
        assert!(m.authority.is_empty());
    }

    fn version_of(s: &str) -> (String, i64) {
        extract_version(&parse_complete(s).expect("valid json"))
    }

    #[test]
    fn extracts_top_level_version() {
        let (v, major) = version_of(r#"{"version":"2.150.0","name":"drift"}"#);
        assert_eq!(v, "2.150.0");
        assert_eq!(major, 2);
    }

    #[test]
    fn extracts_nested_metadata_version() {
        let (v, major) = version_of(r#"{"metadata":{"name":"x","version":"0.1.0"}}"#);
        assert_eq!(v, "0.1.0");
        assert_eq!(major, 0, "major 0 must be distinct from unknown");
    }

    #[test]
    fn missing_version_reports_unknown_major() {
        let (v, major) = version_of(r#"{"name":"x"}"#);
        assert_eq!(v, "");
        assert_eq!(major, -1);
    }

    #[test]
    fn rejects_chunk_truncated_mid_document() {
        // Shape observed on mainnet at slot 435767459: a chunk write that decompresses
        // cleanly, still contains "version", but is cut off mid-document.
        let partial = r#"{"address":"FUsi","metadata":{"name":"trench_lock","version":"0.1.0"},"instructions":[{"name":"lock","disc"#;
        assert!(
            parse_complete(partial).is_err(),
            "partial chunk must not pass as a version"
        );
    }

    #[test]
    fn accepts_complete_document() {
        let full = r#"{"address":"FUsi","metadata":{"name":"trench_lock","version":"0.1.0"},"instructions":[]}"#;
        let parsed = parse_complete(full).expect("should parse");
        assert_eq!(extract_version(&parsed), ("0.1.0".to_string(), 0));
    }

    #[test]
    fn roundtrips_uncompressed_payload() {
        assert_eq!(decompress(b"{\"a\":1}", 0, MAX_DECOMPRESSED_LEN).unwrap(), "{\"a\":1}");
    }

    #[test]
    fn empty_payload_is_an_error_not_empty_string() {
        assert!(decompress(&[], 2, MAX_DECOMPRESSED_LEN).is_err());
    }

    #[test]
    fn rejects_compression_bomb_instead_of_exhausting_memory() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
        let chunk = vec![b'a'; 1024 * 1024];
        for _ in 0..(MAX_DECOMPRESSED_LEN / chunk.len() + 2) {
            enc.write_all(&chunk).unwrap();
        }
        let bomb = enc.finish().unwrap();
        assert!(bomb.len() < 100_000, "bomb should be small on the wire: {}", bomb.len());

        let err = decompress(&bomb, 2, MAX_DECOMPRESSED_LEN).expect_err("should refuse to inflate past the cap");
        assert!(err.contains("exceeds"), "{err}");
        // The point is not that it was refused but that it was refused *early*: the
        // reader stops one byte past the allowance instead of materialising the bomb.
        assert!(
            err.contains(&format!("read {}", MAX_DECOMPRESSED_LEN + 1)),
            "decompression must stop at the allowance, not after inflating: {err}"
        );
    }

    /// Every other test builds its fixture from METADATA_PROGRAM_ID, so a wrong
    /// constant would still pass while the metadata branch matched nothing on chain.
    #[test]
    fn metadata_program_id_matches_its_documented_base58() {
        assert_eq!(
            bs58::encode(METADATA_PROGRAM_ID).into_string(),
            "ProgM6JCCvbYkfKqJYHePx4xxSUSqJp7rh8Lyv7nk7S"
        );
    }

    #[test]
    fn rejects_gzip_bomb_too_not_only_zlib() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut enc = GzEncoder::new(Vec::new(), Compression::best());
        let chunk = vec![b'a'; 256 * 1024];
        for _ in 0..(MAX_DECOMPRESSED_LEN / chunk.len() + 2) {
            enc.write_all(&chunk).unwrap();
        }
        let bomb = enc.finish().unwrap();
        let err = decompress(&bomb, 1, MAX_DECOMPRESSED_LEN).expect_err("should refuse");
        assert!(err.contains("exceeds"), "{err}");
        assert!(err.contains(&format!("read {}", MAX_DECOMPRESSED_LEN + 1)), "{err}");
    }

    #[test]
    fn roundtrips_gzip() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(br#"{"ok":true}"#).unwrap();
        let packed = enc.finish().unwrap();
        assert_eq!(decompress(&packed, 1, MAX_DECOMPRESSED_LEN).unwrap(), r#"{"ok":true}"#);
    }

    #[test]
    fn a_smaller_allowance_rejects_a_payload_the_cap_would_allow() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&vec![b'a'; 4096]).unwrap();
        let packed = enc.finish().unwrap();

        assert!(decompress(&packed, 2, MAX_DECOMPRESSED_LEN).is_ok());
        let err = decompress(&packed, 2, 1024).expect_err("spent allowance must bite");
        assert!(err.contains("allowance"), "{err}");
    }

    #[test]
    fn rejects_unknown_compression_rather_than_guessing_zlib() {
        let err = decompress(b"whatever", 7, MAX_DECOMPRESSED_LEN).expect_err("should reject");
        assert!(err.contains("unsupported compression 7"), "{err}");
    }

    #[test]
    fn metadata_truncated_payload_reports_declared_and_actual() {
        let mut data = vec![0u8; 200];
        data[0] = METADATA_DISCRIMINATOR_METADATA;
        data[87..91].copy_from_slice(&5000u32.to_le_bytes());

        let err = parse_metadata(&data).expect_err("should reject");
        assert!(err.contains("5000"), "{err}");
        assert!(err.contains(&(200 - METADATA_HEADER_LEN).to_string()), "{err}");
    }
}
