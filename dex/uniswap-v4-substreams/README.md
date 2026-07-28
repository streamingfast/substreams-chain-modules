# Uniswap v4 Substreams (Base)

sg2ss conversion of [Uniswap/v4-subgraph](https://github.com/Uniswap/v4-subgraph) for **Base**.

| Layer | Role |
|-------|------|
| **Substreams** | Decode PoolManager + PositionManager events (address-gated) |
| **ClickHouse** | Event tables + HOL views (`api_pools`, `api_swaps`, …) |
| **Hasura** | GraphQL over public `api_*` views |

**No Substreams stores** for Tier A+B — v4 pool events all come from a fixed PoolManager.

**Source:** [streamingfast/substreams-chain-modules · dex/uniswap-v4-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/dex/uniswap-v4-substreams)

**Showcase:** [substreams.dev/showcases/uniswap-v4](https://substreams.dev/showcases/uniswap-v4)

See [DESIGN.md](./DESIGN.md).

## Contracts (Base)

| Role | Address | startBlock |
|------|---------|------------|
| PoolManager | `0x498581fF718922c3f8e6A244956aF099B2652b2b` | 25350988 |
| PositionManager | `0x7C5f5A4bBd8fD63184577525326123B519429bDc` | 25350993 |

## Prerequisites

Install the [Substreams CLI](https://docs.substreams.dev/how-to-guides/installing-the-cli) (includes `substreams sink clickhouse` — [migration guide](https://docs.substreams.dev/how-to-guides/sinks/sql/migration)). Docker for the local ClickHouse + Hasura stack.

## Build

From the monorepo root (workspace member):

```bash
cd substreams-chain-modules
cargo build -p uniswap_v4_substreams --release --target wasm32-unknown-unknown
# or:
cd dex/uniswap-v4-substreams && substreams build
# → uniswap-v4-substreams-v0.1.0.spkg
```

## Get running

```bash
git clone https://github.com/streamingfast/substreams-chain-modules.git
cd substreams-chain-modules/dex/uniswap-v4-substreams

docker compose up -d                    # ClickHouse + Hasura + connector
# substreams auth                       # once

substreams sink clickhouse ./substreams.yaml \
  --dsn "clickhouse://default:SecureMe!@127.0.0.1:9000/uniswap_v4" \
  -s 25350988 \
  --network base \
  --batch-block-flush-interval 50 \
  --bytes-encoding hex \
  --cursor-file-path ./cursor.txt \
  --sink-info-folder ./ch-sink-info

# after event tables exist:
./scripts/apply_views.sh
./scripts/hasura_connect_clickhouse.sh
```

Then open the Console or hit GraphQL:

| Service | URL |
|---------|-----|
| GraphQL | http://localhost:8080/v1/graphql · header `x-hasura-admin-secret: SecureMe!` |
| Console | http://localhost:8080/console |
| ClickHouse | http://127.0.0.1:8123 · DB `uniswap_v4` |

```bash
curl -s http://localhost:8080/v1/graphql \
  -H 'content-type: application/json' \
  -H 'x-hasura-admin-secret: SecureMe!' \
  -d '{"query":"{ uniswap_v4_api_pools(limit: 5) { pool_id token0 token1 fee tick liquidity swap_count } }"}'
```

More examples: [`sql/hasura/examples.graphql`](./sql/hasura/examples.graphql)

**Hosted option:** StreamingFast can host the sink against your warehouse — [hosted sinks](https://docs.substreams.dev/how-to-guides/sinks) · [Portal](https://app.streamingfast.io).

## API views

| View | Meaning |
|------|---------|
| `api_pools` | Pool id HOL: tokens, fee, latest price/tick/liquidity |
| `api_swaps` | Swap events |
| `api_modify_liquidity` | Liquidity modifications |
| `api_positions` | Position NFT latest owner |
| `api_tokens` | Currency addresses seen (no symbol/decimals eth_call yet) |

## Out of core (for now)

- USD volumes / Bundle pricing  
- Tick entities, day/hour aggregates as full subgraph parity  
- Hook factory extras (Arrakis/Euler)  
- Token metadata eth_call  
