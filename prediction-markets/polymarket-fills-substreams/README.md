# Polymarket Fills Substreams

**Source:** [streamingfast/substreams-chain-modules · prediction-markets/polymarket-fills-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/prediction-markets/polymarket-fills-substreams)

Unified Polymarket order-fill history on Polygon (chain ID 137): CLOB v1
(CTF Exchange, Neg Risk CTF Exchange) merged with CLOB v2
([`colindickson/polymarket-exchange`](https://substreams.dev/packages/polymarket-exchange/latest))
into one ordinal-sorted `OrderFilled`/`OrdersMatched` stream with an
`exchange_version` column, so a single query spans both eras.

## Why this exists

Two third-party Polymarket packages were pulled from substreams.dev because
they didn't work:

- `polymarket-orderbook-substreams` had no block-level index filter on any
  fill-extraction module, so it scanned and billed every block in range
  regardless of whether it contained a matching event.
- `polymarket-pnl`'s `map_order_fills` mixed checksummed contract addresses
  into an `eth_common:index_events` filter that is lowercase-keyed, so the
  filter never matched anything — verified against 2,000 real fills over
  blocks 93,000,000–93,002,000 that `map_order_fills` returned zero rows for.

This package fixes both: every fill-extraction module has a lowercase-address
`eth_common:index_events` block filter (verified with `substreams info`), and
output was cross-checked against `colindickson/polymarket-exchange`'s
`map_all_events` over an identical block range (138,447 `OrderFilled` +
50,418 `OrdersMatched`, exact match).

## Contracts

| Contract | Address | Deploy block | Verified |
|---|---|---|---|
| CTF Exchange (v1) | `0x4bfb41d5b3570defd03c39a9a4d8de6bd8b8982e` | 33,605,403 | `eth_getCode` binary search against `https://137.rpc.thirdweb.com`, 2026-09 |
| Neg Risk CTF Exchange (v1) | `0xc5d563a36ae78145c45a50134d48a1215220f80a` | 50,505,492 | same method |
| CTF Exchange (v2) | `0xe111180000d2663c0091e4f400237545b87b996b` | 84,902,353 | from `colindickson/polymarket-exchange` |

Neither v1 deploy block was taken from prior documentation — both were
re-derived from Polygon RPC because an earlier, unsourced figure
(57,000,000) had propagated through both packages this replaces.

## Modules

| Module | Kind | Output |
|---|---|---|
| `map_v1_events` | map | `polymarket.fills.v1.V1Events` — native decode of the two v1 contracts |
| `map_fills` | map | `polymarket.fills.v1.UnifiedFills` — `OrderFilled`/`OrdersMatched`/`OrderCancelled`, v1 + v2 |
| `map_fee_events` | map | `polymarket.fills.v1.UnifiedFeeEvents` |
| `map_admin_events` | map | `polymarket.fills.v1.UnifiedAdminEvents` |
| `map_pause_events` | map | `polymarket.fills.v1.UnifiedPauseEvents` |

V2's `side` + `token_id` encoding is normalized into `maker_asset_id` /
`taker_asset_id` (side `BUY` (0): maker's asset is `"0"`, i.e. collateral;
side `SELL` (1): reversed — confirmed against
[`Polymarket/ctf-exchange-v2`](https://github.com/Polymarket/ctf-exchange-v2)
`Structs.sol`/`CalculatorHelper.sol`), so v1 and v2 rows union without a
schema mismatch.

v1 has no equivalent for v2's `FeeReceiverUpdated`, `MaxFeeRateUpdated`,
`OrderPreapproved`/`OrderPreapprovalInvalidated`, or per-user
`UserPauseBlockIntervalUpdated` — those fields are only ever populated from
the v2 leg. v1's global `TradingPaused`/`TradingUnpaused` are normalized into
`UserPaused`/`UserUnpaused` with an empty `user` (v1 has no per-user pause
targeting).

## Quickstart

```bash
substreams run polymarket-fills-substreams@v0.1.0 map_fills -e polygon --start-block -1
```

## Validation

```bash
# 1. Every fill-extraction module is index-filtered on lowercase addresses.
substreams info ./polymarket-fills-substreams-v0.1.0.spkg | grep -A2 "Name: map_"

# 2. Non-empty v2-era output, cross-checked against colindickson/polymarket-exchange.
substreams run ./polymarket-fills-substreams-v0.1.0.spkg map_fills -e polygon --start-block 93000000 --stop-block +2000 -o jsonl

# 3. Non-empty v1-era output.
substreams run ./polymarket-fills-substreams-v0.1.0.spkg map_fills -e polygon --start-block 40000000 --stop-block +2000 -o jsonl

# 4. Processed-block density is well below 100% during a quiet historical range.
substreams estimate ./polymarket-fills-substreams-v0.1.0.spkg map_fills -e polygon --start-block 33605403 --stop-block 45000000 --yes
```

Built with [Substreams Skills](https://github.com/streamingfast/substreams-skills)
(`substreams-dev`, `substreams-ethereum`).
