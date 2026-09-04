# Uniswap v4 → Substreams (ClickHouse) — Design

**Status:** package in `dex/uniswap-v4` (sg2ss)  
**Networks:** every chain with a Uniswap v4 deployment (see README)  
**Sink:** ClickHouse relational mappings via `substreams sink clickhouse` (insert-only)  
**Upstream subgraph:** [Uniswap/v4-subgraph](https://github.com/Uniswap/v4-subgraph)  
**Placement:** [substreams-chain-modules](https://github.com/streamingfast/substreams-chain-modules) · `dex/uniswap-v4`  
**Tier:** **A + B** (event feed + core HOL: Pool, Swap, ModifyLiquidity, Position ownership)

## Principles (sg2ss)

| # | Rule |
|---|------|
| P1 | Events in WASM; state in ClickHouse |
| P2 | **No business stores.** V4 core logs all emit from fixed **PoolManager** / **PositionManager** — store module optional (reserved for later dynamic hooks) |
| P3 | HOL / volumes / day-data in SQL only |
| P4 | ClickHouse insert-only relational mappings (`substreams sink clickhouse`) |
| P5 | Outcome parity, not GraphQL schema parity |
| P6 | Deterministic block/time fields |
| P7 | Filter `_deleted_ = false` |
| P8 | DB filled only by Substreams |

## Architecture note (V4 vs V3)

Uniswap **v4 pools are not separate contracts**. All pool lifecycle events (`Initialize`, `Swap`, `ModifyLiquidity`, …) are emitted by a single **PoolManager**. Pool identity is `bytes32` **pool id**, not a template address.

→ **No dynamic pool-address store required for Tier A+B.**  
Address-gate only the fixed PoolManager + PositionManager (ERC-721 `Transfer` is generic — must not be topic-only).

## Per-network contracts

`map_events` is address-gated on the chain's PoolManager and PositionManager, both
supplied as params (`pool_manager=0x...&position_manager=0x...`). The manifest's
`networks:` block sets those params and the matching `initialBlock` for each supported
network; `--network` picks the entry. Addresses track `networks.json` in v4-subgraph —
see the README table for the full list.

## Modules

| Module | Kind | Role |
|--------|------|------|
| `map_events` | map | Decode PoolManager + PositionManager logs → typed events |

No `store_dynamic_*` in v0.1 (can add later for optional hook factories / token metadata allowlists).

## Event surface (Tier A)

**PoolManager (gated):**

- `Initialize` — new pool id + currencies + fee + tickSpacing + hooks + initial price/tick  
- `Swap`  
- `ModifyLiquidity`  
- (later) `Donate`, protocol fee events  

**PositionManager (gated):**

- `Transfer` — NFT ownership (position tokenId)  
- `Subscription` / `Unsubscription`  

## HOL (Tier B) — ClickHouse

| View | Outcome |
|------|---------|
| `api_pools` | One row per pool id: token0/1, fee, tick_spacing, hooks, latest sqrt_price / tick / liquidity from Initialize + Swap + ModifyLiquidity |
| `api_swaps` | Swap events (analytics) |
| `api_modify_liquidity` | Liquidity change events |
| `api_positions` | Latest owner per position `token_id` from PositionManager Transfer |
| `api_tokens` | Distinct currency addresses seen on Initialize (metadata eth_call **out of core**) |

**Out of Tier B (later):**

- USD pricing / `volumeUSD` / `Bundle` (subgraph eth_call + pricing utils)  
- Tick entities, day/hour candles as full subgraph parity  
- Arrakis / Euler hook factories  

## Parity checklist (core)

1. Set of pool ids after Initialize matches subgraph for a block range  
2. Latest pool sqrtPrice / tick / liquidity vs subgraph `Pool` for sample ids  
3. Swap count in a block range  
4. Position owner for sample tokenIds  

## Implementation phases

| Phase | Status |
|-------|--------|
| A — map_events package | this package |
| B — sink tables | ops |
| C — api_* HOL SQL | this package |
| D — parity harness | follow-up |
| G — Hasura | optional, same as superfluid stack pattern |
