# Polymarket PnL Substreams

**Source:** [streamingfast/substreams-chain-modules · prediction-markets/polymarket-pnl-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/prediction-markets/polymarket-pnl-substreams)

Per-address Polymarket positions, mark-to-market PnL, volume and whale
detection on Polygon, derived from
[`streamingfast/polymarket-fills-substreams`](https://substreams.dev/packages/polymarket-fills-substreams/latest)
(unified CLOB v1 + v2 fills), plus CTF position splits, merges and payout
redemptions from
[`colindickson/polymarket-ctf`](https://substreams.dev/packages/polymarket-ctf/latest)
— including a market registry. Ships a PostgreSQL `db_out` sink (Database
Changes) for the `trades`, `whale_alerts`, `user_positions`, `user_pnl`,
`token_prices` and `markets` tables. The package is mappers only — no
stores: Postgres accumulates the running totals.

This supersedes `PaulieB14/polymarket-pnl` — same feature set, but its
`map_order_fills` never actually produced fill data (a checksummed-vs-
lowercase address mismatch against its `eth_common:index_events` filter),
so nothing downstream of it ever ran on real rows.

## PnL model: mark-to-market, not average cost

`total_pnl = net_cash_flow + token_amount * latest_price`. This is exact —
no averaging assumption — and once `token_amount` reaches `0` it *is* the
fully realized PnL. It does mean realized and unrealized PnL are not
reported as two separate numbers; that would need lot-level tracking, which
needs state a mapper does not have.

## No stores: Postgres does the accumulating

Every module is a stateless mapper. `db_out` emits per-block *deltas* as
Database Changes `add` ops (`col = COALESCE(col, 0) + value`) onto
`user_positions.token_amount`, `user_positions.net_cash_flow`,
`user_pnl.total_volume` and `user_pnl.net_cash_flow`, and the latest fill
price per token as a `set` on `token_prices`. Rows touched several times in
one block are summed before they reach the sink.

`total_pnl` is nonlinear in the running quantity, so it cannot be
accumulated as a delta and is not a column. It is the `user_positions_pnl`
view, which joins `user_positions` to `token_prices`. Because the join
happens at query time, every holder is re-marked when a price moves — not
only the ones who traded since.

Consequences:

- **Postgres only.** `add` ops need the Postgres sink; ClickHouse cannot
  apply them.
- **Not idempotent.** Absolute values could be rewritten safely; deltas
  cannot. Restarting from the sink's saved cursor is fine, but re-running a
  block range against a database that already contains it double-counts
  `user_positions` and `user_pnl` (the `trades` and `whale_alerts` inserts
  fail on their primary key first). Reset the database before replaying.
- **Reorgs** are reverted by the sink's history handling, so run it with
  reorg handling on (leave `--undo-buffer-size` at its default of `0`) or
  only ingest final blocks.
- **A partial range starts from zero.** Started mid-history, sold tokens
  acquired earlier show as negative `token_amount`. Backfill from block
  `33605403` for correct positions.

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
token, and only when it is non-zero). `map_redemption_legs` therefore takes
`colindickson/polymarket-ctf`'s `map_erc1155_events` as well and reads the
amount closed from the ERC-1155 burn (`TransferSingle` to the zero address)
the same call emitted just before `PayoutRedemption`. Each redemption is
matched to the nearest preceding unconsumed burn of the same
(transaction, redeemer, token ID), so a merge of the same token earlier in
the transaction is never mistaken for it. Index sets the redeemer held
nothing in emit no burn and are skipped; the payout is spread evenly over
the tokens that were burned.

That even spread is an approximation: the event does not say which outcome
won, so a redeemer holding both a winning and a losing token has the payout
attributed half-and-half rather than all to the winner. Per-user totals
(`user_pnl`, and the sum of `user_positions_pnl.total_pnl`) are unaffected;
per-token `net_cash_flow` is. Weighting by `markets.payout_numerators`
would fix it.

Splits/merges attribute cost/proceeds evenly across the resulting
partition entries (see `even_shares` in `src/lib.rs`) — this makes a
split-then-immediately-merge round trip net to exactly zero cash flow,
regardless of how many outcomes the partition covers.

## Modules

| Module | Kind | Output |
|---|---|---|
| `map_trade_legs` | map | `polymarket.pnl.v1.TradeLegs` — per-fill-leg position/cash-flow deltas, volume, price, trades, whale alerts |
| `map_ctf_legs` | map | `polymarket.pnl.v1.TradeLegs` — CTF split (mint) / merge (burn) legs |
| `map_redemption_legs` | map | `polymarket.pnl.v1.TradeLegs` — CTF redemption (close) legs, amount read from the matching ERC-1155 burn |
| `map_markets` | map | `polymarket.pnl.v1.MarketDeltas` — condition metadata + resolution |
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
