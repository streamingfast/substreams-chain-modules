# Superfluid Substreams — Design

**Status:** implementation in progress (v0.2.3 package + ClickHouse CFA/IDA/GDA HOL views)  
**Network (v1):** Base mainnet  
**Sink:** ClickHouse (from-proto / insert-only)  
**Source of truth (business):** chain events → DB  
**Source of truth (dynamic contracts):** Substreams stores (addresses only)

---

## 1. Purpose

Index the Superfluid Protocol so applications can obtain the **same outcomes** as the official subgraph (streams for an account, current rates, pool membership, etc.) without requiring:

- Substreams stores for HOL or aggregates  
- A 1:1 conversion of the subgraph GraphQL schema  

Structure may differ; **query outcomes** must match for an agreed parity set.

Upstream reference:  
[superfluid-org/protocol-monorepo `packages/subgraph`](https://github.com/superfluid-org/protocol-monorepo/tree/dev/packages/subgraph)

---

## 2. Design principles

| # | Principle |
|---|-----------|
| P1 | **Events in Substreams, state in the database.** WASM decodes logs; SQL/views answer “what is true now?” |
| P2 | **Stores only for dynamic discovery.** Track SuperToken / Pool / governance addresses so maps know which logs to decode. No stream rates, units, or snapshots in stores. |
| P3 | **Outcome parity, not schema parity.** Different tables/views are fine if apps can derive the same answers. |
| P4 | **ClickHouse insert-only.** No Database Changes / UPDATE semantics; current state = projections over event history (views / optional MVs). |
| P5 | **Deterministic backfill.** Prefer `block_number` + `log_index` + `block_timestamp` from the chain over wall-clock `now()` except for “live streamed amount” queries. |
| P6 | **Reorg-safe reads.** Views filter sink tombstones (`_deleted_ = 0` or equivalent). |

---

## 3. Architecture

```
                    ┌─────────────────────────────────────┐
                    │            Base mainnet             │
                    │         (Firehose / Substreams)     │
                    └──────────────────┬──────────────────┘
                                       │
                    ┌──────────────────▼──────────────────┐
                    │         Substreams package          │
                    │                                     │
                    │  store_dynamic_addresses            │
                    │    • SuperToken addrs (factory)     │
                    │    • Pool addrs (GDA PoolCreated)   │
                    │    • Governance addrs (optional)    │
                    │                                     │
                    │  map_events                         │
                    │    • fixed protocol contracts       │
                    │    • dynamic SuperToken / Pool logs │
                    │    • emit typed event protos only   │
                    └──────────────────┬──────────────────┘
                                       │ from-proto sink
                    ┌──────────────────▼──────────────────┐
                    │            ClickHouse               │
                    │                                     │
                    │  L0  Event tables (append-only)     │
                    │  L1  Key / normalize views          │
                    │  L2  Lifecycle views (streams…)     │
                    │  L3  Aggregate views                │
                    │  L4  API views (stable app surface) │
                    └──────────────────┬──────────────────┘
                                       │
                    ┌──────────────────▼──────────────────┐
                    │  Apps / BI / Hasura GraphQL (read)  │
                    └─────────────────────────────────────┘
```

**Not in Substreams:** Stream/Index/Pool entity machines, AccountTokenSnapshot math, running totals.

**In ClickHouse:** all of the above as views (or MVs that only read events).

**GraphQL:** optional [Hasura](https://hasura.io/graphql/database/clickhouse) layer over ClickHouse (`api_*` views). Read-oriented; does not write protocol state.  
Public `api_*` views + track list + examples: **`sql/hasura/`**. Internal HOL: `sql/clickhouse/`. Ops: `docker-compose.yml` + `scripts/hasura_connect_clickhouse.sh`.

**Hard rule:** ClickHouse is filled **only** by Substreams (`map_events` → `substreams-sink-sql from-proto`). No GraphQL seed scripts, no manual registry inserts from the subgraph.

---

## 4. Substreams package

### 4.1 Modules

| Name | Kind | Inputs | Output | Rules |
|------|------|--------|--------|--------|
| `store_dynamic_addresses` | store (`set_if_not_exists`, `int64`) | Block | — | Keys only: `st:{addr}`, `pool:{addr}`, `gov:{addr}` → `1` |
| `map_events` | map | Block + store (get) | `superfluid.v1.Events` | Decode & emit events; **no** derived HOL rows |

Optional later: `index_*` / `blockFilter` via `ethereum-common` for cost — still no business stores.

### 4.2 Store allowlist (only dynamic stuff)

| Key prefix | Written by | Used by |
|------------|------------|---------|
| `st:` | `SuperTokenCreated`, `CustomSuperTokenCreated` only | Accept SuperToken logs (Transfer, upgrade, liquidations, …) |
| `pool:` | GDA `PoolCreated` | Accept SuperfluidPool logs |
| `gov:` | Initial known gov address + `GovernanceReplaced` | Accept governance logs if address changes |

**Hard rule:** ClickHouse is populated **only** by Substreams (`map_events` → from-proto). No GraphQL seed scripts, no manual registry inserts.

**Forbidden store contents:** flow rates, deposits, stream ids, units, balances, counters, “current” anything.

### 4.3 Fixed contracts (Base mainnet)

From Superfluid metadata (`base-mainnet`, `startBlockV1` / `initialBlock`: **1_000_000**):

| Role | Address |
|------|---------|
| Host | `0x4C073B3baB6d8826b8C5b229f3cfdC1eC6E47E74` |
| CFA | `0x19ba78B9cDB05A877718841c574325fdB53601bb` |
| IDA | `0x66DF3f8e14CF870361378d8F61356D15d9F425C4` |
| GDA | `0xfE6c87BE05feDB2059d2EC41bA0A09826C9FD7aa` |
| SuperTokenFactory | `0xe20B9a38E0c96F61d1bA6b42a61512D56Fea1Eb3` |
| Resolver | `0x6a214c324553F96F04eFBDd66908685525Da0E0d` |
| TOGA | `0xA87F76e99f6C8Ff8996d14f550ceF47f193D9A09` |
| Governance (seed) | `0x55F7758dd99d5e185f4CC08d4Ad95B71f598264D` |

Addresses compared as **lowercase hex without `0x`** (or consistently with `0x` — pick one package-wide).

### 4.4 Event coverage (map_events)

Emit one row per relevant log (subgraph event surface), including at least:

- **CFA:** FlowUpdated, FlowUpdatedExtension, FlowOperatorUpdated  
- **IDA:** Index* / Subscription* events  
- **GDA:** PoolCreated, PoolConnectionUpdated, BufferAdjusted, Instant/FlowDistributionUpdated  
- **Pool:** MemberUnitsUpdated, DistributionClaimed  
- **Factory:** SuperTokenCreated, CustomSuperTokenCreated, SuperTokenLogicCreated  
- **SuperToken (dynamic addr):** Transfer, Approval, Minted, Burned, TokenUpgraded/Downgraded, AgreementLiquidated*  
- **Host / Resolver / Governance / TOGA:** as in subgraph handlers  

**Enrichment policy (v1):**  
- Prefer fields **on the log**.  
- Subgraph-only eth_call fields (`realtimeBalanceOfNow`, CFA `getFlow` deposit extras, …) are **out of core parity** unless added later as an optional path.

### 4.5 Common event columns

Every event row should include:

| Column | Purpose |
|--------|---------|
| `id` | Primary key, e.g. `{tx_hash}-{log_index}` |
| `block_number`, `block_timestamp` | Ordering & time |
| `transaction_hash`, `log_index` | Traceability / order within block |
| `contract` | Emitter address |
| `event_name` | Discriminator |

Plus event-specific fields (`token`, `sender`, `receiver`, rates as decimal strings, etc.).

ClickHouse: from-proto annotations (`schema.table`, `order_by` with PK prefix, partition by month on injected `_block_timestamp_` where applicable). Avoid reserved CH names (`index` → `index_id`, etc.).

---

## 5. Database layers

### L0 — Event tables

Physical tables populated by the sink. Append-only (+ reorg metadata).  
Source of truth for everything above.

### L1 — Normalized keys

Thin views that:

- Lowercase addresses  
- Define `stream_pair_key = sender || '-' || receiver || '-' || token`  
- Unify token/pool ids  

### L2 — Lifecycle / HOL projections

Views that answer subgraph-like entity questions.

#### 5.1 Stream (CFA) — primary teaching model

**Subgraph meaning of Stream:** one CFA lifetime for `(sender, receiver, token)` until closed; reopen → new revision.

**DB derivation** from ordered `flow_updated_events` per pair:

| Event class | Rule (conceptual) |
|-------------|-------------------|
| Create | new rate ≠ 0 after 0 / first open |
| Update | rate stays non-zero |
| Terminate | new rate = 0 |

```text
revision = cumulative creates (or terminate→create cycles) along the pair timeline
stream_id = sender || '-' || receiver || '-' || token || '-' || revision
```

**Views (illustrative names):**

| View | Outcome |
|------|---------|
| `v_flow_updates_enriched` | Each FlowUpdated + `event_type`, `revision`, `stream_id` |
| `v_streams_current` | One row per `stream_id`: current rate, deposit, opened/updated/closed, streamed_until_updated_at |
| `v_stream_periods` | Constant-rate segments between updates |
| `v_account_streams` | Filter helper: streams where account is sender or receiver |

**Streamed amount:**

- At last event: sum of `rate * Δt` over closed periods + partial math as in subgraph mappings  
- As of timestamp **T** (open streams):  
  `streamed_amount_as_of(streamed_until_updated_at, current_flow_rate, updated_at, T, is_open)`  
  which is `streamed_until_updated_at + current_flow_rate * (T - updated_at)` when open  
- `api_streams.streamed_as_of_now` uses wall-clock `now()` only as a live convenience — **not** for Phase D parity (use fixed H / T)

#### 5.2 Other HOL (same pattern)

| Outcome | Built from | Projection style |
|---------|------------|------------------|
| Account | any address column on events | distinct + first/last seen |
| Token | factory create (+ resolver listed if modeled) | one row per token addr |
| FlowOperator | FlowOperatorUpdated | `argMax` / latest per (operator, sender, token) |
| Index | IDA events | latest value/units + sum distribution deltas |
| IndexSubscription | approve/revoke/claim/units + IndexSubscribed/Unsubscribed | units, approved, subscribed, amount received + pending |
| Pool | GDA + member units | flow_rate, buffer, unit stats, instant + flowed distributed |
| PoolMember | MemberUnits + connection + claims | units, connected, total_claimed |
| PoolDistributor | flow/instant/buffer events | flow_rate, buffer, distributed totals |
| TokenGovernance | governance events | latest per superToken (incl. default) |

**Default “current state” SQL pattern:**

```sql
argMax(column, (block_number, log_index))
GROUP BY business_key
WHERE _deleted_ = 0
```

**Running totals:** sum of delta events, or last absolute field when the log already carries totals.

### L3 — Aggregates

| Outcome | Approx. subgraph | SQL idea |
|---------|------------------|----------|
| Account × token activity | AccountTokenSnapshot (core) | From `v_streams_current` + GDA membership: active counts, sum rates, net rate |
| Token-wide | TokenStatistic (core) | Aggregations over streams / distribution events |

**Core parity (in scope):** structure, rates, event-derived totals.  
**Balance / liquidation eth_call fields:** optional tier 2, not required for v1 design.

### L4 — API views

Stable names for applications, e.g.:

- `api_streams_for_account(account)`  
- `api_stream_by_id(stream_id)`  
- `api_account_token_summary(account, token)`  

These hide internal L1–L3 renames and protect apps when physical tables evolve.

### Materialized views

Allowed as **performance** caches of the same pure-SQL definitions.  
Still fed only by event tables — not by Substreams stores.

---

## 6. Outcome parity

### Definition

For a fixed block height **H** (or timestamp **T**), database API views and the subgraph GraphQL API should agree on the **parity checklist** within documented tolerances (e.g. 1 second on open-stream amounts).

### Checklist (v1 core)

| # | Outcome |
|---|---------|
| 1 | Set of streams for an account (in + out), open vs closed |
| 2 | Current flow rate & deposit per open stream |
| 3 | Streamed amount at H for sample streams |
| 4 | IDA index current value / units for sample indexes |
| 5 | GDA pool members & units for sample pools |
| 6 | Counts of major event types in a block range |

### Explicitly optional / later

| Outcome | Why |
|---------|-----|
| SuperToken Transfer history full parity | Costly without careful filtering; store helps address set but Transfer volume is high |
| realtimeBalanceOfNow | eth_call |
| Liquidation estimate timestamps | often balance-dependent |

---

## 7. Mapping from subgraph concepts

| Subgraph layer | This design |
|----------------|-------------|
| Event entities | L0 event tables (`map_events`) |
| HOL entities | L2 views (not WASM state) |
| Aggregates | L3 views |
| Data source templates | `store_dynamic_addresses` + filter in `map_events` |
| eth_call in handlers | Out of core parity |
| GraphQL schema | L4 API views / app layer (not Graph Node) |

### “Stream” (product language)

A **Stream** is one Superfluid **CFA** continuous flow lifetime:

- **sender → receiver** in one **Super Token**  
- Updated in place while open; **closed** when rate goes to 0  
- **Reopened** later ⇒ new stream identity (new revision)  

“All streams for this account” = all such lifecycles where the account is sender or receiver.

---

## 8. Implementation phases

| Phase | Deliverable |
|-------|-------------|
| **A** | Confirm package: stores = dynamic only; `map_events` events only; build/pack |
| **B** | ClickHouse: event tables live; reorg columns understood |
| **C** | CFA L1–L2 views: enriched flows, current streams, periods, account streams |
| **D** | Parity harness: sample accounts vs subgraph at block H |
| **E** | IDA views |
| **F** | GDA + pool views |
| **G** | L3 aggregates + L4 API views |
| **H** | Optional: transfer strategy, eth_call tier, multi-chain |

**Implemented in repo (v0.2.3):**

- Phase **A**: `store_dynamic_addresses` + `map_events` only; package builds
- Phase **C–G (SQL)**: ClickHouse views under `sql/clickhouse/` (CFA streams, IDA/GDA/tokens, API views)
- **P0 correctness:** fixed-contract address gates (Host/Resolver/CFA/IDA/GDA/Factory/TOGA — kills OZ AccessControl Role\* pollution); multi-flow deposit pairing by ordinal (not ASOF); `streamed_amount_as_of` for parity-safe as-of-H amounts
- **P2 IDA/GDA depth:** IndexSubscription amount-received accrual + Approve/Revoke + IndexSubscribed/Unsubscribed; Index distributed totals; GDA Pool (flow_rate, buffer, instant/flowed distributed, unit stats), PoolDistributor, member claims

**Still open:** Phase B full sink backfill, Phase D CFA parity at fixed block H, GDA member particle RTB fine points, multi-chain, ATS/token metadata/GraphQL product surface.

---

## 9. Non-goals

- Bitwise-identical GraphQL types/fields  
- Substreams stores as a general-purpose entity DB  
- Postgres Database Changes for mutable rows (ClickHouse path chosen)  
- Hosted deploy process details (separate runbook; output quality gate before hosted sink)

---

## 10. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Stream revision SQL bugs | Golden tests vs subgraph on known accounts; unit SQL on fixture event sequences |
| Query latency on heavy views | MVs; proper ORDER BY / projections; limit open-ended scans |
| SuperToken Transfer cost | Core product on CFA/IDA/GDA first; transfers as opt-in |
| Short `substreams run` without factory history | Seed well-known SuperTokens in store |
| Schema drift vs Superfluid upgrades | Version package; track monorepo ABIs |
| Generic-topic pollution (Role\*/Set/Host) | Address-gate fixed contracts in `map_events` + store discovery |
| Multi-flow deposit mis-pair | Ordinal join FU↔FUE within tx (not ASOF-only) |
| Parity vs `now()` | `streamed_amount_as_of(..., H_ts, ...)` for fixed-height compares |

---

## 11. Decision log

| Decision | Choice |
|----------|--------|
| Chain | Base mainnet first (eth-mainnet addresses retired in v0.2.3) |
| Sink | ClickHouse, from-proto |
| Business state | DB views / SQL / optional MVs |
| Substreams stores | **Only** dynamic addresses |
| Schema vs subgraph | May differ |
| Success metric | Outcome parity matrix |
| eth_call | Not required for core parity |
| Multi-chain | Later, same pattern |

---

## 12. Glossary (short)

| Term | Meaning |
|------|---------|
| **CFA** | Constant Flow Agreement (streams) |
| **IDA** | Instant Distribution Agreement (indexes) |
| **GDA** | General Distribution Agreement (pools) |
| **HOL** | Higher-order entity (long-lived business object) |
| **ATS** | Account–token snapshot (aggregate) |
| **spkg** | Built Substreams package |
| **Stream** | One CFA lifetime sender→receiver→token (+ revision) |

---

## 13. Next steps (when leaving design-only)

1. Align repo modules with §4 (trim store to allowlist; no HOL in WASM).  
2. Document L0 table list = event types actually emitted.  
3. Implement Phase **C** SQL (CFA views) against a CH with event data.  
4. Run Phase **D** parity on 2–3 known Superfluid accounts.  
5. Expand IDA/GDA only after CFA outcomes match.

---

*This document is the design baseline from the Wrangler session constraints: dynamic stores only, aggregations in the database, outcome parity with the Superfluid subgraph.*
