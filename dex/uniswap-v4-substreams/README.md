# Uniswap v4 Substreams

sg2ss conversion of [Uniswap/v4-subgraph](https://github.com/Uniswap/v4-subgraph), running on every chain where Uniswap v4 is deployed.

| Layer | Role |
|-------|------|
| **Substreams** | Decode PoolManager + PositionManager events (address-gated) |
| **ClickHouse** | Event tables + HOL views (`api_pools`, `api_swaps`, …) |
| **Hasura** | GraphQL over public `api_*` views |

**No Substreams stores** for Tier A+B — v4 pool events all come from a single PoolManager per chain.

**Source:** [streamingfast/substreams-chain-modules · dex/uniswap-v4-substreams](https://github.com/streamingfast/substreams-chain-modules/tree/main/dex/uniswap-v4-substreams)

**Showcase:** [substreams.dev/showcases/uniswap-v4](https://substreams.dev/showcases/uniswap-v4)

See [DESIGN.md](./DESIGN.md).

## Networks

The chain is selected with `--network`; the manifest carries the matching contract
addresses (as `map_events` params) and `initialBlock` for each one.

| Chain | `--network` | PoolManager | PositionManager | initialBlock |
|-------|-------------|-------------|-----------------|--------------|
| Ethereum | `mainnet` | `0x000000000004444c5dc75cB358380D2e3dE08A90` | `0xbD216513d74C8cf14cf4747E6AaA6420FF64ee9e` | 21688329 |
| Base | `base` | `0x498581fF718922c3f8e6A244956aF099B2652b2b` | `0x7C5f5A4bBd8fD63184577525326123B519429bDc` | 25350988 |
| Arbitrum One | `arbitrum-one` | `0x360E68faCcca8cA495c1B759Fd9EEe466db9FB32` | `0xd88F38F930b7952f2DB2432Cb002E7abbF3dD869` | 297842872 |
| OP Mainnet | `optimism` | `0x9a13F98Cb987694C9F086b1F5eB990EeA8264Ec3` | `0x3C3Ea4B57a46241e54610e5f022E5c45859A1017` | 130947675 |
| Polygon | `matic` | `0x67366782805870060151383F4BbFF9daB53e5cD6` | `0x1Ec2eBf4F37E7363FDfe3551602425af0B3ceef9` | 66980384 |
| BNB Smart Chain | `bsc` | `0x28e2Ea090877bF75740558f6BFB36A5ffeE9e9dF` | `0x7A4a5c919aE2541AeD11041A1AEeE68f1287f95b` | 45970610 |
| Avalanche | `avalanche` | `0x06380C0e0912312B5150364B9DC4542BA0DbBc85` | `0xB74b1F14d2754AcfcbBe1a221023a5cf50Ab8ACD` | 56195376 |
| Unichain | `unichain` | `0x1F98400000000000000000000000000000000004` | `0x4529A01c7A0410167c5740C487A8DE60232617bf` | 25500 |
| World Chain | `worldchain` | `0xb1860D529182ac3BC1F51Fa2ABd56662b7D13f33` | `0xC585E0f504613b5fBf874F21Af14c65260fB41fA` | 9111872 |
| Blast | `blast-mainnet` | `0x1631559198A9e474033433b2958daBC135ab6446` | `0x4AD2F4CcA2682cBB5B950d660dD458a1D3f1bAaD` | 14377311 |
| Zora | `zora` | `0x0575338e4C17006aE181B47900A84404247CA30f` | `0xf66C7b99e2040f0D9b326B3b7c152E9663543D63` | 25434534 |
| Soneium | `soneium` | `0x360e68faccca8ca495c1b759fd9eee466db9fb32` | `0x1b35d13a2e2528f192637f14b05f0dc0e7deb566` | 2473300 |
| Ink | `ink` | `0x360e68faccca8ca495c1b759fd9eee466db9fb32` | `0x1b35d13a2e2528f192637f14b05f0dc0e7deb566` | 4580556 |
| Celo | `celo` | `0x288dc841A52FCA2707c6947B3A777c5E56cd87BC` | `0xf7965f3981e4D5BC383BfBCb61501763e9068CA9` | 43985160 |
| Linea | `linea` | `0x248083fb965359d82b06c1f5322480dcfc1ad857` | `0xddcad5775b2816a87495f207731b3571d7ee3c76` | 28974969 |
| Monad | `monad` | `0x188d586Ddcf52439676Ca21A244753fA19F9Ea8e` | `0x5b7eC4a94fF9beDb700fb82aB09d5846972F4016` | 29255895 |
| X Layer | `xlayer-mainnet` | `0x360E68faCcca8cA495c1B759Fd9EEe466db9FB32` | `0xcF1EAFC6928dC385A342E7C6491d371d2871458b` | 46106401 |
| MegaETH | `megaeth` | `0xacb7e78fa05d562e0a5d3089ec896d57d057d38e` | `0x9ae0921e981aaa7308f176f8d4f9129b9247c89d` | 7009653 |
| Tempo | `tempo` | `0x33620f62c5b9b2086dd6b62f4a297a9f30347029` | `0x3fc79444f8eacc1894775493ff3fa41f1e35ce11` | 6606180 |
| Robinhood Chain | `robinhood` | `0x8366a39cc670b4001a1121b8f6a443a643e40951` | `0x58daec3116aae6d93017baaea7749052e8a04fa7` | 9070 |

Testnets:

| Chain | `--network` | PoolManager | PositionManager | initialBlock |
|-------|-------------|-------------|-----------------|--------------|
| Ethereum Sepolia | `sepolia` | `0xE03A1074c86CFeDd5C142C4F04F1a1536e203543` | `0x429ba70129df741B2Ca2a85BC3A2a3328e5c09b4` | 7258946 |
| Base Sepolia | `base-sepolia` | `0x05E73354cFDd6745C338b50BcFDfA3Aa6fA03408` | `0x4b2c77d209d3405f41a037ec6c77f7f5b8e2ca80` | 19088197 |
| Arbitrum Sepolia | `arbitrum-sepolia` | `0xFB3e0C6F74eB1a21CC1Da29aeC80D2Dfe6C9a317` | `0xAc631556d3d4019C95769033B5E719dD77124BAc` | 105909222 |
| Unichain Sepolia | `unichain-testnet` | `0x00b036b58a818b1bc34d502d3fe730db729e62ac` | `0xf969aee60879c54baaed9f3ed26147db216fd664` | 7092034 |

Addresses come from [`networks.json`](https://github.com/Uniswap/v4-subgraph/blob/main/networks.json)
in the upstream subgraph; `initialBlock` is the earlier of the two deployment blocks.

Uniswap v4 is also deployed on Arc, which has no Substreams endpoint yet.

To point the module at contracts not listed here, override the params directly:

```bash
substreams run ./substreams.yaml map_events \
  --network base \
  -p map_events="pool_manager=0x498581ff718922c3f8e6a244956af099b2652b2b&position_manager=0x7c5f5a4bbd8fd63184577525326123b519429bdc"
```

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
