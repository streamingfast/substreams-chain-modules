# Solana IDL History

Chain-wide index of Anchor IDL versions on Solana.

Solana keeps no history of a program's IDL: the account holds only the current
one, and reading an older version means finding the transaction that wrote it.
That makes parsing historical transactions against the IDL of the day
impractical. This package indexes every IDL write as it happens, so a program's
versions stay queryable by major version and by time.

It takes no parameters — one deployment covers every program on the chain.

## Formats

Two on-chain formats carry IDLs:

- **legacy** — an account at `create_with_seed(base, "anchor:idl", program)`.
  `create_with_seed` takes the program as the owner, so the account is owned by
  the program it describes and attribution needs no lookup.
- **SPL Program Metadata** (`ProgM6JCCvbYkfKqJYHePx4xxSUSqJp7rh8Lyv7nk7S`),
  adopted by Anchor 1.0.0, which names its program in a 96-byte header. That
  program is a general-purpose store keyed by a seed — `security` is another
  standard one — so only accounts under the `idl` seed are indexed. Without that
  filter a `security` blob carrying a `version` field would be recorded as a
  program version that never existed.

Legacy accounts cannot be filtered for: the accounts foundational module indexes
only exact `account:` and `owner:` keys, and every program owns its own IDL
account. So the package reads the raw `AccountBlock` and matches the `IdlAccount`
discriminator in WASM, which also keeps the slot and block time that the
filtered stream drops.

## Modules

| module | kind | output |
|---|---|---|
| `index_idl_accounts` | blockIndex | `sf.substreams.index.v1.Keys` |
| `map_idl_accounts` | map | `sf.solana.idl.v1.IdlAccountWrites` |

The block index is what makes a chain-wide scan affordable: IDL writes are rare,
so the map reads only the blocks that touch one.

Anchor writes an IDL in chunks, and the intermediate writes hold truncated
documents that still carry a `version` field. Every write is emitted, but only
the ones whose payload parses as a whole JSON object are marked `complete`, so a
consumer can tell a real version from a partial upload.

Nothing on chain authenticates an IDL account, so every payload is untrusted
input. A single decompressed payload is capped at 1 MiB and a whole block at
8 MiB, because the text is then parsed into a value tree costing many times its
length; strings bound for the sink's `VARCHAR(255)` columns are stripped of NULs
and cut to length. All of it exists so that one crafted account cannot abort the
module, which on a chain-wide index would poison that block on every replay.

## Build and run

From the workspace root:

```bash
cargo build --target wasm32-unknown-unknown --release -p solana_idl_history_substreams
```

Then from this directory:

```bash
substreams pack
```

`src/pb` is generated and checked in. Regenerate it with `substreams protogen`
after changing a proto.

```bash
substreams run -e accounts.mainnet.sol.streamingfast.io:443 \
  solana-idl-history-substreams-v0.2.0.spkg map_idl_accounts \
  -s -1000 --limit-processed-blocks 0
```

`--limit-processed-blocks` defaults to 10000 as a guard against accidental
reprocessing; `0` lifts it for a real backfill.

## Sink

`substreams-sink-sql from-proto` builds the `idl_write` table straight from the
annotations on `sf.solana.idl.v1.IdlAccountWrite`. There is no schema file and no
migration — the sink owns the table shape.

Two consequences of that path shaped the proto:

- It allows a single primary-key column, so `(account_address, slot)` is encoded
  into a synthetic `id`.
- It maps every proto `string` to `VARCHAR(255)`, and Postgres rejects rather
  than truncates on overflow. A real IDL runs to hundreds of kilobytes, so
  `idl_json` is `bytes`, which maps to unbounded `BYTEA`. Hosted deployments
  cannot set `--bytes-encoding`, so the stored value is proto-JSON base64 with
  its surrounding quotes and the reader has to unwrap it.

## Coverage

The Solana accounts stream does not reach genesis — it currently starts around
slot 327,404,500. Programs whose IDL was last written before that are not in the
index; reaching them means replaying `anchor:idl` instructions from the
transaction stream, which is a separate pipeline.

## Showcase

The site built on this package lives in
[streamingfast/solana-idl-history](https://github.com/streamingfast/solana-idl-history).
