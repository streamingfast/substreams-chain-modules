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
| `store_ref_price` | store (set, string) | `map_chainlink_answers` | `ref:<ticker>` → `<answer_usd>\|<updated_at>\|<block_ts>` |
| `store_session_close` | store (set, string) | `map_stock_swaps` | `close:<ticker>` → `<price_usd>\|<block_ts>`, last priced swap during a regular NYSE session (weekday 09:30–16:00 America/New_York, exchange holidays excluded; a static holiday table through 2027, with early closes at 13:00 on the days the exchange shortens the session). |
| `map_basis` | map | `map_stock_swaps`, both stores | `hood.basis.v1.BasisTicks` — per priced swap: implied vs reference (`chainlink`, else `session_close`, else `none`), `premium_bps`, and the session (`regular` / `extended` / `closed`). |
| `db_out` | map | `Clock`, the three maps | `sf.substreams.sink.database.v1.DatabaseChanges` for `stock_swaps`, `chainlink_answers`, `basis_ticks`; `stock_registry` is written once at block 9070. |

Every module starts at block 9070, the PoolManager deployment block. The
upstream package's stores must see every pool `Initialize`, so the first run
of any module that depends on `u4rh` backfills from 9070; use
`--production-mode` for anything beyond a smoke test.

Static inputs live in `data/` and are baked into the wasm by `build.rs`:
`registry-4663.tsv` (194 registry assets with ERC-8056 multipliers) and
`feed-token-map.tsv` (35 ticker → token → Chainlink proxy → aggregator rows).

## Build

```bash
make protogen   # only after editing proto/ or substreams.yaml imports
make build      # cargo build --target wasm32-unknown-unknown --release -p robinhood_stock_tokens_substreams
make pack       # substreams pack -> robinhood-stock-tokens-substreams-v0.1.0.spkg
make test       # cargo test -p robinhood_stock_tokens_substreams
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
inserts only). `substreams-sink-sql` cannot insert into ClickHouse `Decimal`
columns in DatabaseChanges mode, so every exact value is inserted as
`<name>_str String` and `<name> Decimal(38,18)` is `MATERIALIZED` from it;
query the `Decimal` column. It also defines two rollups: `basis_hourly` (avg/min/max premium, last
implied and reference price, swap count, volume) and `swaps_hourly` (buys,
sells, volume, shares). The rollups are `AggregatingMergeTree` /
`SummingMergeTree` materialized views; query them through `basis_hourly_v`
and `swaps_hourly_v`, which apply the `-Merge` step.

```bash
substreams-sink-sql setup "clickhouse://default:@localhost:9000/default" ./robinhood-stock-tokens-substreams-v0.1.0.spkg
substreams-sink-sql run   "clickhouse://default:@localhost:9000/default" ./robinhood-stock-tokens-substreams-v0.1.0.spkg --undo-buffer-size 12
```

## Conventions

- Addresses are `0x`-prefixed lowercase; amounts are decimal strings, never floats.
- `price_usd`, `implied_usd`, `answer_usd` are truncated to 18 decimals with trailing zeros removed.
- `side` is from the trader's point of view: `buy` when the trader receives stock.
- Unpriced swaps have empty `amount_usd` / `price_usd` in the proto and `0` with `priced = false` in ClickHouse.
- `stock_registry` also carries `decimals` and `status` from the registry snapshot.
- `map_basis` reads the stores with `get_first`, so every swap in a block is compared against the reference as it stood at the start of the block, never against a value written earlier in that same block.
