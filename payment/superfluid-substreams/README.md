# Superfluid on Substreams

**[Substreams](https://substreams.streamingfast.io) is the engine** — parallel WASM modules stream Superfluid protocol events from Base. Sink them into [ClickHouse](https://clickhouse.com), then serve apps with [Hasura](https://hasura.io) GraphQL. Built with [StreamingFast](https://streamingfast.io).

**Source:** [streamingfast/substreams-chain-modules · payment/superfluid-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/payment/superfluid-substreams) — clone this package before running.

**Marketing page (showcases):** [docs/superfluid](https://github.com/streamingfast/showcases/tree/master/docs/superfluid) · open [`index.html`](https://github.com/streamingfast/showcases/blob/master/docs/superfluid/index.html) from the [showcases](https://github.com/streamingfast/showcases) repo.

### Why Substreams

Substreams sits at the center of this package: deterministic decoding, parallel backfill with module caching, and a portable `.spkg` you can sink anywhere. ClickHouse and Hasura consume Substreams output — they don’t re-scan the chain.

1. **Substreams extract** — WASM modules decode CFA / IDA / GDA / factory logs (address-gated, typed protobuf).
2. **Substreams sink** — `substreams-sink-sql from-proto` inserts events into ClickHouse (append-only, reorg-aware).
3. **Serve Substreams data** — SQL views compute streams, pools, memberships; Hasura exposes GraphQL.

Coming from a **subgraph**? Same Superfluid surface, different indexing engine (Substreams instead of Graph Node). You still get GraphQL for apps — plus raw SQL analytics and a reusable Substreams package. Business HOL lives in SQL views, not WASM stores.

### Query Superfluid streams in GraphQL

With the stack up (`docker compose up -d`, sink running, views + Hasura track applied):

```bash
curl -s http://localhost:8080/v1/graphql \
  -H 'content-type: application/json' \
  -H 'x-hasura-admin-secret: SecureMe!' \
  -d '{"query":"{ superfluid_api_streams(limit: 5, where: { is_open: { _eq: 1 } }) { stream_id sender receiver token current_flow_rate deposit } }"}'
```

```graphql
query OpenStreams {
  superfluid_api_streams(limit: 5, where: { is_open: { _eq: 1 } }) {
    stream_id
    sender
    receiver
    token
    current_flow_rate   # Super Token base units per second
    deposit
  }
}
```

**Console:** [http://localhost:8080/console](http://localhost:8080/console) · admin secret `SecureMe!`  
**More examples:** [`sql/hasura/examples.graphql`](./sql/hasura/examples.graphql)

### Get running

**Package path:** [github.com/streamingfast/substreams-chain-modules/tree/main/payment/superfluid-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/payment/superfluid-substreams)

Clone the chain-modules repo and run everything from the package directory:

```bash
git clone https://github.com/streamingfast/substreams-chain-modules.git
cd substreams-chain-modules/payment/superfluid-substreams
docker compose up -d                    # ClickHouse + Hasura
# substreams auth                       # once
substreams-sink-sql from-proto \
  clickhouse://default:SecureMe!@127.0.0.1:9000/superfluid \
  ./substreams.yaml map_events \
  -s 1000000 --network base \
  --bytes-encoding hex \
  --clickhouse-cursor-file-path ./cursor.txt \
  --clickhouse-sink-info-folder ./ch-sink-info
# after tables exist:
./scripts/apply_views.sh
./scripts/hasura_connect_clickhouse.sh
```

Then open the Console or hit `/v1/graphql` as above.

**Hosted option:** StreamingFast can also **host the Substreams sink** for you (managed runner against your ClickHouse or Postgres). See [hosted sinks](https://docs.substreams.dev/how-to-guides/sinks) and the [StreamingFast Portal](https://app.streamingfast.io).

---

## Package details

**Network:** Base mainnet · **initialBlock:** `1000000` · **Version:** `v0.2.3`

| Module | Kind | Role |
|--------|------|------|
| `store_dynamic_addresses` | store | SuperToken / Pool / gov **addresses only** |
| `map_events` | map | Typed protocol events → ClickHouse from-proto |

Business HOL (streams, pools, …) is **not** in Substreams stores — it lives in SQL. See [DESIGN.md](./DESIGN.md).

### Build

```bash
cd superfluid
substreams build
# → superfluid-v0.2.3.spkg
```

### Smoke run (events only)

```bash
substreams auth
substreams run ./substreams.yaml map_events \
  --network base \
  -s 1000000 -t +200 \
  -o jsonl
```

### Full local stack

**All protocol rows come from Substreams** (`from-proto`). No GraphQL seeds into ClickHouse.

```bash
docker compose up -d

# optional clean DB
curl -u 'default:SecureMe!' 'http://127.0.0.1:8123/' --data 'DROP DATABASE IF EXISTS superfluid'
curl -u 'default:SecureMe!' 'http://127.0.0.1:8123/' --data 'CREATE DATABASE superfluid'

export SUBSTREAMS_API_TOKEN=…   # or substreams auth
substreams-sink-sql from-proto \
  clickhouse://default:SecureMe!@127.0.0.1:9000/superfluid \
  ./substreams.yaml map_events \
  -s 1000000 \
  --network base \
  --block-batch-size 50 --bytes-encoding hex \
  --clickhouse-cursor-file-path ./cursor.txt \
  --clickhouse-sink-info-folder ./ch-sink-info

./scripts/apply_views.sh
./scripts/hasura_connect_clickhouse.sh

# optional parity vs official Superfluid subgraph
SUPERFLUID_SUBGRAPH=https://subgraph-endpoints.superfluid.dev/base-mainnet/protocol-v1 \
  ./scripts/parity_cfa.sh
```

| Service | URL / DSN |
|---------|-----------|
| ClickHouse HTTP | `http://127.0.0.1:8123` · user `default` · password `SecureMe!` · DB `superfluid` |
| ClickHouse native | `clickhouse://default:SecureMe!@127.0.0.1:9000/superfluid` |
| Hasura Console | http://localhost:8080/console |
| GraphQL | http://localhost:8080/v1/graphql · header `x-hasura-admin-secret: SecureMe!` |

### SQL layout

| Path | Role |
|------|------|
| `sql/clickhouse/` | Internal HOL (`v_*`) |
| [`sql/hasura/01_api_cfa.sql`](./sql/hasura/01_api_cfa.sql) | Public `api_*` CFA views |
| [`sql/hasura/02_api_ida_gda.sql`](./sql/hasura/02_api_ida_gda.sql) | Public `api_*` IDA/GDA/token views |
| [`sql/hasura/tracklist.sql`](./sql/hasura/tracklist.sql) | Views Hasura tracks |
| [`sql/hasura/examples.graphql`](./sql/hasura/examples.graphql) | Sample GraphQL |

### Main API views

| View | Outcome |
|------|---------|
| `api_streams` | Current CFA streams (+ live `streamed_as_of_now`; prefer `streamed_amount_as_of` for parity) |
| `api_account_token_cfa_stats` | Active stream counts & rates per account×token |
| `api_flow_operators` | Latest flow operator permissions |
| `api_tokens` | SuperTokens from factory |
| `api_indexes` / `api_index_subscriptions` | IDA HOL: value/units, amount received + pending |
| `api_pools` | GDA pool HOL: units, flow_rate, buffer, distributed totals |
| `api_pool_members` / `api_pool_distributors` | Membership & distributors |
| `api_accounts` | Addresses seen on major events |

GraphQL root fields look like `superfluid_api_streams` (`{database}_{view}`).

### Correctness notes

- Fixed-contract events are **address-gated** (no generic AccessControl pollution).
- CFA deposit joins pair FlowUpdated ↔ Extension by **ordinal within the tx**.
- Parity-safe streamed amounts: `streamed_amount_as_of(..., <unix_ts_H>, is_open)` — not only wall-clock `now()`.

### Contracts (Base mainnet)

| Role | Address |
|------|---------|
| Host | `0x4C073B3baB6d8826b8C5b229f3cfdC1eC6E47E74` |
| CFA | `0x19ba78B9cDB05A877718841c574325fdB53601bb` |
| IDA | `0x66DF3f8e14CF870361378d8F61356D15d9F425C4` |
| GDA | `0xfE6c87BE05feDB2059d2EC41bA0A09826C9FD7aa` |
| SuperTokenFactory | `0xe20B9a38E0c96F61d1bA6b42a61512D56Fea1Eb3` |
| Resolver | `0x6a214c324553F96F04eFBDd66908685525Da0E0d` |
| TOGA | `0xA87F76e99f6C8Ff8996d14f550ceF47f193D9A09` |
| Governance | `0x55F7758dd99d5e185f4CC08d4Ad95B71f598264D` |

### Out of core (for now)

- eth_call balances / liquidation estimates  
- Full global ERC-20 Transfer indexing (only SuperTokens in the dynamic store)  
- Bitwise-identical GraphQL schema vs the Superfluid subgraph  

### Design

[DESIGN.md](./DESIGN.md) — architecture, store allowlist, parity matrix, phases.

---

*Built with [StreamingFast](https://streamingfast.io) Substreams · showcase for [streamingfast/showcases](https://github.com/streamingfast/showcases).*
