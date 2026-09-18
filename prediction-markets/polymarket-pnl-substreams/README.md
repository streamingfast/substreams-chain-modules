# Polymarket PnL Substreams

**Source:** [streamingfast/substreams-chain-modules · prediction-markets/polymarket-pnl-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/prediction-markets/polymarket-pnl-substreams)

Per-address Polymarket positions, mark-to-market PnL, volume and whale
detection on Polygon, derived from
[`streamingfast/polymarket-fills-substreams`](https://substreams.dev/packages/polymarket-fills-substreams/latest)
(unified CLOB v1 + v2 fills) plus a market registry from
[`colindickson/polymarket-ctf`](https://substreams.dev/packages/polymarket-ctf/latest).
Ships a PostgreSQL `db_out` sink (Database Changes) for the `trades`,
`whale_alerts`, `user_positions`, `user_pnl` and `markets` tables.

This supersedes `PaulieB14/polymarket-pnl` — same feature set, but its
`map_order_fills` never actually produced fill data (a checksummed-vs-
lowercase address mismatch against its `eth_common:index_events` filter),
so nothing downstream of it ever ran on real rows.

## PnL model: mark-to-market, not average cost

`user_positions.total_pnl = net_cash_flow + token_amount * latest_price`.
This is exact — no averaging assumption — and once `token_amount` reaches
`0` it *is* the fully realized PnL. It does mean realized and unrealized
PnL are not reported as two separate numbers.

The reason is structural, not a shortcut: average-cost accounting needs a
module that reads its own prior output (to know the running average before
writing the next delta), and Substreams' store engine does not support
that — accumulation for `updatePolicy: add`/`set` stores happens natively
in the runtime, never inside your own module, so a module can feed a store
or read a store but never both for the same store. Every store this package
writes (`store_user_positions`, `store_user_cash_flow`, `store_user_volume`,
`store_market_volume`, `store_latest_prices`) is a plain commutative
accumulation for exactly this reason, keeping the module graph acyclic.

## Scope: CTF splits/merges/redemptions are not in v0.1.0

`colindickson/polymarket-ctf`'s `PositionSplit`, `PositionsMerge` and
`PayoutRedemption` events carry a `condition_id` and an index set, not a
token ID. Recovering the token ID means implementing Gnosis CTF's
`getCollectionId`, which for Polymarket's actual deployment is an
elliptic-curve (BN254) point encode/decode with a Tonelli-Shanks-style
modular square root — not the plain keccak concatenation older CTF
deployments use. Shipping that without a way to verify it against a known
answer is exactly the kind of "looks plausible, quietly wrong" arithmetic
that this package's sibling (`polymarket-fills-substreams`, replacing a
broken lowercase/checksum address filter) exists to avoid repeating. CTF is
used only for the `markets` registry (condition metadata + resolution
status), which needs no such derivation.

## Modules

| Module | Kind | Output |
|---|---|---|
| `map_trade_legs` | map | `polymarket.pnl.v1.TradeLegs` — per-fill-leg position/cash-flow deltas, volume, price, trades, whale alerts |
| `map_markets` | map | `polymarket.pnl.v1.MarketDeltas` — condition metadata + resolution |
| `store_user_positions` | store (add, bigint) | `<user>:<token_id>` → net open position |
| `store_user_cash_flow` | store (add, bigint) | `<user>:<token_id>` and `<user>:ALL` → signed cumulative cash flow |
| `store_user_volume` | store (add, bigint) | `<user>` → lifetime notional traded |
| `store_market_volume` | store (add, bigint) | `<token_id>` → lifetime notional traded |
| `store_latest_prices` | store (set, proto) | `<token_id>` → most recent trade price |
| `db_out` | map | `sf.substreams.sink.database.v1.DatabaseChanges` |

## Whale detection

Any fill leg with a collateral amount ≥ 10,000 USDC (`10_000_000_000` raw
atomic units) is written to `whale_alerts`, one row per side of the trade.

## Run it

```bash
substreams build
export SUBSTREAMS_SINK_DSN="postgres://user:pass@localhost:5432/polymarket?sslmode=disable"
substreams sink postgres setup polymarket-pnl-substreams-v0.1.0.spkg
substreams sink postgres polymarket-pnl-substreams-v0.1.0.spkg
```

Built with [Substreams Skills](https://github.com/streamingfast/substreams-skills)
(`substreams-dev`, `substreams-sql`).
