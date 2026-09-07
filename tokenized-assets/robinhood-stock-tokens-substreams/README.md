# Robinhood Stock Tokens Substreams

**Source:** [streamingfast/substreams-chain-modules · tokenized-assets/robinhood-stock-tokens-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/tokenized-assets/robinhood-stock-tokens-substreams)

**Showcase:** https://github.com/streamingfast/robinhood-showcase

Substreams package for Robinhood Chain (Arbitrum Orbit L2, chain id 4663) that
measures the onchain basis of tokenized stocks: the price implied by each
Uniswap V4 stock-token swap against a reference price, per trade, sunk to
ClickHouse.

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
| `db_out` | map | the three maps | `sf.substreams.sink.database.v1.DatabaseChanges` for `stock_swaps`, `chainlink_answers`, `basis_ticks`. `stock_registry` is not emitted here; it is seeded once by the INSERT in `schema.clickhouse.sql`. |

Every module starts at block 9070, the PoolManager deployment block. The
upstream package's stores must see every pool `Initialize`, so the first run
of any module that depends on `u4rh` backfills from 9070; use
`--production-mode` for anything beyond a smoke test.

Static inputs live in `data/` and are baked into the wasm by `build.rs`:
`registry-4663.tsv` (194 registry assets with ERC-8056 multipliers) and
`feed-token-map.tsv` (35 ticker → token → Chainlink proxy → aggregator rows).
The same two files feed `scripts/gen-registry-sql.sh`, which generates the
`stock_registry` INSERT in `schema.clickhouse.sql`.

## Build

```bash
substreams protogen   # only after editing proto/ or substreams.yaml imports

cargo build --target wasm32-unknown-unknown --release -p robinhood_stock_tokens_substreams   # from repo root
cargo test -p robinhood_stock_tokens_substreams                                               # from repo root

substreams pack                                          # from the package dir
substreams info ./robinhood-stock-tokens-substreams-v0.1.0.spkg
```

## Run

```bash
substreams auth   # once; needs a Substreams API key

substreams run ./robinhood-stock-tokens-substreams-v0.1.0.spkg map_basis \
  -e robinhood.substreams.pinax.network:443 -s 52700000 -t +5000

substreams run ./robinhood-stock-tokens-substreams-v0.1.0.spkg map_stock_swaps \
  -e robinhood.substreams.pinax.network:443 -s 52700000 -t +5000 -o jsonl

substreams run ./robinhood-stock-tokens-substreams-v0.1.0.spkg map_chainlink_answers \
  -e robinhood.substreams.pinax.network:443 -s 52700000 -t +5000 -o jsonl
```

## Sink

`schema.clickhouse.sql` defines the four tables (`ReplacingMergeTree`,
inserts only), plus a one-time INSERT that seeds `stock_registry` from
`data/registry-4663.tsv` and `data/feed-token-map.tsv` (regenerate it with
`scripts/gen-registry-sql.sh` after editing either file). `substreams-sink-sql`
cannot insert into ClickHouse `Decimal` columns in DatabaseChanges mode, so
every exact value is inserted as `<name>_str String` and `<name>
Decimal(38,18)` is `MATERIALIZED` from it; query the `Decimal` column.

It also defines two hourly rollups, `basis_hourly_v` (avg/min/max premium,
last implied and reference price, swap count, volume, by `ref_source`) and
`swaps_hourly_v` (buys, sells, volume, shares known). Since the base tables
are `ReplacingMergeTree`, both are plain views that read with `FINAL` rather
than materialized views holding partial aggregate state, so they stay
correct across a cursor replay. ClickHouse has no reorg handling in this
sink, so replaying from an earlier cursor is the recovery path after a
reorg.

```bash
substreams-sink-sql setup "clickhouse://default:@localhost:9000/default" ./robinhood-stock-tokens-substreams-v0.1.0.spkg
substreams-sink-sql run   "clickhouse://default:@localhost:9000/default" ./robinhood-stock-tokens-substreams-v0.1.0.spkg --undo-buffer-size 12
```

## Conventions

- Addresses are `0x`-prefixed lowercase; amounts are decimal strings, never floats.
- `price_usd`, `implied_usd`, `answer_usd` are truncated to 18 decimals with trailing zeros removed.
- `side` is from the trader's point of view: `buy` when the trader receives stock.
- Unpriced swaps have empty `amount_usd` / `price_usd` in the proto and `0` with `priced = false` in ClickHouse. `shares_known` is a separate flag for whether `shares_ui` was populated upstream; an unpriced swap can still have known shares, and vice versa.
- `stock_registry` also carries `decimals`, `status` and `snapshot_date` from the registry snapshot; it is seeded once by the INSERT in `schema.clickhouse.sql`, not written per block.
- `map_basis` reads the stores with `get_first`, so every swap in a block is compared against the reference as it stood at the start of the block, never against a value written earlier in that same block.

## Caveats

- Chainlink aggregators are matched to a ticker by aggregator address from a 2026-09-05 snapshot of `data/feed-token-map.tsv`. If Chainlink rotates the aggregator behind a feed, that ticker's reference price stops updating; watch for `ref_ts` no longer advancing as the symptom.
- The NYSE holiday table in `session.rs` only covers 2026-2027 and needs extending before it runs out; past that it will misclassify holidays as regular trading sessions.
- Share counts and `premium_bps` for the 11 tickers with a non-1.0 ERC-8056 multiplier depend on the multiplier baked in from the imported `uniswap-v4-robinhood` package's own snapshot; if that package's multipliers change upstream, this package's values drift until it re-imports.
- The imported `uniswap-v4-robinhood` spkg is fetched by URL in `substreams.yaml` with no checksum pin, so it is not reproducible from source alone. The published `.spkg` for this package, which embeds that import, is the reproducible artifact - not a from-source rebuild.
