# velodrome-substreams

Substreams package for [Velodrome V2](https://velodrome.finance) on Optimism. Indexes pool creation, swaps, and liquidity events, outputting `DatabaseChanges` for SQL sink consumption.

## Contracts

| Contract | Address | Init Block |
|----------|---------|------------|
| PoolFactory | `0xF1046053aa5682b4F9a81b5481394DA16BE5FF5a` | 119142390 |

## Module Graph

```
sf.ethereum.type.v2.Block
        │
        ▼
map_factory_events      ← PoolCreated events from PoolFactory
        │
        ▼
store_pools             ← set_if_not_exists; keys = pool addresses
        │ (get mode)
        ▼
map_pool_events         ← Swap / Mint / Burn filtered to known pools
        │
        ▼
db_out                  ← DatabaseChanges for SQL sink
```

## Output Tables

| Table | Primary Key | Description |
|-------|-------------|-------------|
| `pools` | `address` | Pool creation events (token0, token1, stable flag) |
| `swaps` | `{tx_hash}-{log_index}` | Swap events with amounts in/out |
| `liquidity_events` | `{tx_hash}-{log_index}` | Mint and burn events |

See [schema.sql](schema.sql) for full column definitions.

## Usage

```bash
# Run against Optimism
substreams run velodrome-substreams-v0.1.0.spkg map_factory_events \
  -e optimism.streamingfast.io:443 \
  -s 119142390 -t +200 -o jsonl

# Pipe to PostgreSQL sink
substreams-sink-postgres run \
  "host=localhost user=dev password=dev dbname=velodrome" \
  velodrome-substreams-v0.1.0.spkg db_out
```

## Build

```bash
substreams build
substreams pack substreams.yaml
```

## License

[Apache 2.0](../../../LICENSE)
