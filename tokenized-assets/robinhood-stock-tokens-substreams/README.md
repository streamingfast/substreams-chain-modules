# Robinhood Stock Tokens Substreams

**Source:** [streamingfast/substreams-chain-modules · tokenized-assets/robinhood-stock-tokens-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/tokenized-assets/robinhood-stock-tokens-substreams)

**Showcase:** https://github.com/streamingfast/robinhood-showcase

Substreams package for Robinhood Chain (Arbitrum Orbit L2, chain id 4663) that
measures the onchain basis of tokenized stocks: the price implied by each
Uniswap V4 stock-token swap against a reference price, per trade, sunk to
ClickHouse with `substreams-sink-sql from-proto`.

Built with Substreams Skills (`substreams-dev`, `substreams-sql`).
Reuses [`uniswap-v4-robinhood`](https://github.com/PaulieB14/uniswap-v4-robinhood)
by PaulieB14 for swap decoding, pool/token enrichment, USD pricing and ERC-8056
share counts, and Chainlink tokenized-equity feeds for reference prices.

## Modules

| Module | Kind | Input | Output |
|---|---|---|---|
| `map_stock_swaps` | map | `u4rh:map_stock_events` | `hood.basis.v1.StockSwaps` — one row per swap where one leg is a registry stock and the other is USDG, WETH or native; side, share count, quote amount, USD notional, implied price. Stock/stock pools are counted in `skipped_stock_stock`, anything else in `skipped_other`. |
| `map_chainlink_answers` | map | `sf.ethereum.type.v2.Block` | `hood.basis.v1.ChainlinkAnswers` — `AnswerUpdated` rounds from the 35 tokenized-equity aggregators in `data/feed-token-map.tsv`, answer scaled to 8 decimals. |
| `store_ref_price` | store (set, string) | `map_chainlink_answers` | `ref:<ticker>` → `<answer_usd>\|<updated_at>` |
| `store_session_close` | store (set, string) | `map_stock_swaps` | `close:<ticker>` → `<price_usd>\|<block_ts>`, last priced swap during a regular NYSE session (weekday 09:30–16:00 America/New_York, exchange holidays excluded; a static holiday table through 2027, with early closes at 13:00 on the days the exchange shortens the session). |
| `map_basis` | map | `map_stock_swaps`, both stores | `hood.basis.v1.BasisTicks` — per priced swap: implied vs reference (`chainlink`, else `session_close`, else `none`), `premium_bps`, and the session (`regular` / `extended` / `closed`). |
| `map_rows` | map | the three maps | `hood.basis.v1.Rows` — the sink module. Same rows as the three maps with `id` and `block_time` filled and every decimal column normalised for `Decimal128(18)`; `substreams-sink-sql from-proto` turns its `stock_swaps`, `chainlink_answers` and `basis_ticks` fields into the tables of the same name. |

Every module starts at block 9070, the PoolManager deployment block. The
upstream package's stores must see every pool `Initialize`, so the first run
of any module that depends on `u4rh` backfills from 9070; use
`--production-mode` for anything beyond a smoke test.

Static inputs live in `data/` and are baked into the wasm by `build.rs`:
`registry-4663.tsv` (194 registry assets with ERC-8056 multipliers) and
`feed-token-map.tsv` (35 ticker → token → Chainlink proxy → aggregator rows).

## Build

```bash
substreams protogen   # only after editing proto/ or substreams.yaml imports

cargo build --target wasm32-unknown-unknown --release -p robinhood_stock_tokens_substreams   # from repo root
cargo test -p robinhood_stock_tokens_substreams                                               # from repo root

substreams pack                                          # from the package dir
substreams info ./robinhood-stock-tokens-substreams-v0.2.0.spkg
```

`proto/sf/substreams/sink/sql/schema/v1/schema.proto` is vendored from
[substreams-sink-sql](https://github.com/streamingfast/substreams-sink-sql)
and listed under `protobuf.files` so `substreams pack` bundles it; the
`(schema.table)` / `(schema.field)` options on `basis.proto` are what the
sink reads to create the tables. `substreams protogen` also emits
`src/pb/schema.rs` for it; that file is generated, not hand-written.

## Run

```bash
substreams auth   # once; needs a Substreams API key

substreams run ./robinhood-stock-tokens-substreams-v0.2.0.spkg map_basis \
  -e robinhood.substreams.pinax.network:443 -s 52700000 -t +5000

substreams run ./robinhood-stock-tokens-substreams-v0.2.0.spkg map_stock_swaps \
  -e robinhood.substreams.pinax.network:443 -s 52700000 -t +5000 -o jsonl

substreams run ./robinhood-stock-tokens-substreams-v0.2.0.spkg map_chainlink_answers \
  -e robinhood.substreams.pinax.network:443 -s 52700000 -t +5000 -o jsonl
```

## Sink

The sink creates tables with `CREATE TABLE IF NOT EXISTS` and never alters them. Point it at a fresh database or schema; a database that still holds the v0.1 tables of the same names will accept the setup and then fail on the first insert.

There is no schema file. `substreams-sink-sql from-proto` derives the tables
from the `Rows` message and creates them itself on first run:

```bash
substreams-sink-sql from-proto "clickhouse://default:@localhost:9000/default" \
  ./robinhood-stock-tokens-substreams-v0.2.0.spkg map_rows \
  -e robinhood.substreams.pinax.network:443 --start-block 9070
```

Every table is `ReplacingMergeTree(_version_, _deleted_)`,
`PARTITION BY toYYYYMM(_block_timestamp_)`, `ORDER BY (ticker, block_time, id)`,
with the sink's four bookkeeping columns first. `id` is `<tx_hash>-<log_index>`
and is unique per row; it is part of the sorting key rather than a separate
`PRIMARY KEY` because ClickHouse requires the primary key to be a prefix of the
sorting key and the ticker-first order is what queries need. Columns, exactly
as ClickHouse creates them:

**`stock_swaps`**

| Column | Type |
|---|---|
| `_block_number_`, `_block_timestamp_`, `_version_`, `_deleted_` | `UInt64`, `DateTime`, `Int64`, `Bool` |
| `id` | `String` |
| `block_num`, `block_ts` | `UInt64` |
| `block_time` | `DateTime` |
| `tx_hash` | `String` |
| `log_index` | `UInt32` |
| `pool_id`, `ticker`, `token`, `side` | `String` |
| `shares_ui`, `shares_raw_adjusted` | `Decimal(38, 18)` |
| `quote_symbol`, `quote_kind` | `String` |
| `quote_amount`, `amount_usd`, `price_usd` | `Decimal(38, 18)` |
| `priced`, `shares_known` | `Bool` |
| `sender`, `origin` | `String` |
| `fee` | `UInt32` |
| `hook_address` | `String` |

**`chainlink_answers`**

| Column | Type |
|---|---|
| `_block_number_`, `_block_timestamp_`, `_version_`, `_deleted_` | `UInt64`, `DateTime`, `Int64`, `Bool` |
| `id` | `String` |
| `block_num`, `block_ts` | `UInt64` |
| `block_time` | `DateTime` |
| `tx_hash` | `String` |
| `log_index` | `UInt32` |
| `ticker`, `feed`, `aggregator` | `String` |
| `answer_usd` | `Decimal(38, 18)` |
| `round_id` | `String` |
| `updated_at` | `UInt64` |

**`basis_ticks`**

| Column | Type |
|---|---|
| `_block_number_`, `_block_timestamp_`, `_version_`, `_deleted_` | `UInt64`, `DateTime`, `Int64`, `Bool` |
| `id` | `String` |
| `block_num`, `block_ts` | `UInt64` |
| `block_time` | `DateTime` |
| `tx_hash` | `String` |
| `log_index` | `UInt32` |
| `ticker` | `String` |
| `implied_usd`, `ref_usd` | `Decimal(38, 18)` |
| `ref_source` | `String` |
| `ref_ts` | `UInt64` |
| `premium_bps` | `Int64` |
| `session` | `String` |
| `amount_usd` | `Decimal(38, 18)` |
| `side` | `String` |

Readers use `FINAL`. The sink never updates or deletes in place: a reorg undo
inserts a tombstone (`_deleted_ = true`, higher `_version_`) for every row
above the last valid block, and duplicates from a cursor replay are collapsed
by the same `ReplacingMergeTree` merge. `SELECT ... FROM stock_swaps FINAL
WHERE ticker = 'AAPL'` is the correct read; without `FINAL` a query can see
both the live row and its tombstone until the next merge.

Sink notes:

- The whole `Rows` message is the unit of work per block; `map_rows` emits the
  three lists side by side and the sink inserts each list into its table.
- Decimal columns are never empty: `map_rows` rewrites an unknown value to
  `0`, truncates to 18 decimals and drops any value with more than 20 integer
  digits (it would not fit `Decimal128(18)` and the sink rejects it instead of
  nulling it). `priced` / `shares_known` say whether `amount_usd` / `price_usd`
  and `shares_ui` were real or filled in; `ref_usd` is `0` when `ref_source` is
  `none`.
- Cursor and schema hash are files next to the sink process
  (`cursor.txt`, `<schema>_schema_hash.txt`; `--clickhouse-cursor-file-path`,
  `--clickhouse-sink-info-folder`), not rows in ClickHouse. Keep them with the
  sink's working directory.
- The registry snapshot is baked into
  the wasm from `data/`, and every row already carries `ticker` and `token`.

## Conventions

- Addresses are `0x`-prefixed lowercase; amounts are decimal strings in the protos, never floats.
- `price_usd`, `implied_usd`, `answer_usd` are truncated to 18 decimals with trailing zeros removed.
- `side` is from the trader's point of view: `buy` when the trader receives stock.
- `priced` means upstream valued the swap, so `amount_usd` is real. `price_usd` additionally needs `shares_known`; when shares are unknown it is `0` even on a priced swap. Read the flags, not the zeros.
- `map_basis` reads the stores with `get_first`, so every swap in a block is compared against the reference as it stood at the start of the block, never against a value written earlier in that same block.

## Caveats

- Chainlink aggregators are matched to a ticker by aggregator address from a 2026-09-05 snapshot of `data/feed-token-map.tsv`. If Chainlink rotates the aggregator behind a feed, that ticker's reference price stops updating; watch for `ref_ts` no longer advancing as the symptom.
- The NYSE holiday table in `session.rs` only covers 2026-2027 and needs extending before it runs out; past that it will misclassify holidays as regular trading sessions.
- Share counts and `premium_bps` for the 11 tickers with a non-1.0 ERC-8056 multiplier depend on the multiplier baked in from the imported `uniswap-v4-robinhood` package's own snapshot; if that package's multipliers change upstream, this package's values drift until it re-imports.
- The imported `uniswap-v4-robinhood` spkg is fetched by URL in `substreams.yaml` with no checksum pin, so it is not reproducible from source alone. The published `.spkg` for this package, which embeds that import, is the reproducible artifact - not a from-source rebuild.
