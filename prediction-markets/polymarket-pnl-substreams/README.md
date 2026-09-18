# Polymarket PnL Substreams

**Source:** [streamingfast/substreams-chain-modules · prediction-markets/polymarket-pnl-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/prediction-markets/polymarket-pnl-substreams)

Per-address Polymarket positions, mark-to-market PnL, volume and whale
detection on Polygon, derived from
[`streamingfast/polymarket-fills-substreams`](https://substreams.dev/packages/polymarket-fills-substreams/latest)
(unified CLOB v1 + v2 fills), plus CTF position splits, merges and payout
redemptions from
[`colindickson/polymarket-ctf`](https://substreams.dev/packages/polymarket-ctf/latest)
— including a market registry. Ships a PostgreSQL `db_out` sink (Database
Changes) for the `trades`, `whale_alerts`, `user_positions`, `user_pnl` and
`markets` tables.

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

## CTF position tracking

`colindickson/polymarket-ctf`'s `PositionSplit`, `PositionsMerge` and
`PayoutRedemption` events carry a `condition_id` and an index set, not a
token ID. Recovering the token ID means implementing Gnosis CTF's position-
ID derivation — which, for the general case (nested/combinatorial
positions), is an elliptic-curve (BN254) point encode/decode. That path is
**not** implemented here — it's genuinely unverifiable without a live
reference, which is exactly the kind of "looks plausible, quietly wrong"
arithmetic this package's sibling (`polymarket-fills-substreams`, replacing
a broken lowercase/checksum address filter) exists to avoid repeating.

What *is* implemented, in `src/ctf_position.rs`, is the simpler top-level
case (`parent_collection_id == 0`), ported from Polymarket's own
[`go-ctf-utils`](https://github.com/Polymarket/go-ctf-utils) reference
implementation rather than the on-chain Solidity, and verified two ways:

1. Matches that repo's own test vectors exactly (`cargo test`, see
   `positionid_test.go` for the source vectors).
2. A derived token ID from a real, live `PositionSplit` event was checked
   against real fills data pulled earlier in the same validation pass — it
   matched a `takerAssetId` that actually traded on the exchange for the
   same market, byte for byte.

A sample of 89,328 real `PositionSplit`/`PositionsMerge` events (Polygon
blocks 93,000,000–93,003,000) found **zero** with a non-zero
`parent_collection_id`, and Polymarket's own `go-ctf-utils` has no support
for a non-zero one either — so the top-level-only scope is not expected to
miss anything in practice. Any event with a non-zero `parent_collection_id`
is skipped (not guessed) defensively, in case that ever changes.

`PayoutRedemption` has no per-token amount in the event (Gnosis
`redeemPositions()` burns the caller's *entire* balance of each redeemed
token), so `map_redemption_legs` reads the running position from
`store_user_positions` to know how much to close — reading a store it does
not also feed, which is the only way Substreams allows this without a
module-graph cycle (see `substreams.yaml` module docs for the full
explanation). The closing amount then lands in the separate
`store_user_redemption_adj`; `db_out` sums both stores for the displayed
`user_positions.token_amount`.

Splits/merges attribute cost/proceeds evenly across the resulting
partition entries (see `even_shares` in `src/lib.rs`) — this makes a
split-then-immediately-merge round trip net to exactly zero cash flow,
regardless of how many outcomes the partition covers.

## Modules

| Module | Kind | Output |
|---|---|---|
| `map_trade_legs` | map | `polymarket.pnl.v1.TradeLegs` — per-fill-leg position/cash-flow deltas, volume, price, trades, whale alerts |
| `map_ctf_legs` | map | `polymarket.pnl.v1.TradeLegs` — CTF split (mint) / merge (burn) legs |
| `map_redemption_legs` | map | `polymarket.pnl.v1.TradeLegs` — CTF redemption (close) legs, reads `store_user_positions` |
| `map_markets` | map | `polymarket.pnl.v1.MarketDeltas` — condition metadata + resolution |
| `store_user_positions` | store (add, bigint) | `<user>:<token_id>` → net open position from fills + splits/merges (not redemptions) |
| `store_user_redemption_adj` | store (add, bigint) | `<user>:<token_id>` → redemption-closing adjustment |
| `store_user_cash_flow` | store (add, bigint) | `<user>:<token_id>` and `<user>:ALL` → signed cumulative cash flow |
| `store_user_volume` | store (add, bigint) | `<user>` → lifetime notional traded (fills only) |
| `store_market_volume` | store (add, bigint) | `<token_id>` → lifetime notional traded (fills only) |
| `store_latest_prices` | store (set, proto) | `<token_id>` → most recent trade price (fills only) |
| `db_out` | map | `sf.substreams.sink.database.v1.DatabaseChanges` |

## Whale detection

Any fill leg, CTF split/mint, or redemption with a collateral amount ≥
10,000 USDC (`10_000_000_000` raw atomic units) is written to
`whale_alerts`. Merges are not checked (their split-time leg already was,
and a round-trip split-then-merge isn't a new whale-sized flow).

## Run it

```bash
substreams build
export SUBSTREAMS_SINK_DSN="postgres://user:pass@localhost:5432/polymarket?sslmode=disable"
substreams sink postgres setup polymarket-pnl-substreams-v0.1.0.spkg
substreams sink postgres polymarket-pnl-substreams-v0.1.0.spkg
```

Built with [Substreams Skills](https://github.com/streamingfast/substreams-skills)
(`substreams-dev`, `substreams-sql`).
