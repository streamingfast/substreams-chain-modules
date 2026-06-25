# cryptopunks-substreams

Substreams package indexing CryptoPunks trades and bids on Ethereum mainnet. Outputs `DatabaseChanges` for SQL sink consumption.

- **Network**: Ethereum Mainnet
- **Start block**: 3,914,495 (CryptoPunks contract deployment)
- **Package**: `cryptopunks-substreams-v0.1.0.spkg`

## Modules

| Module | Kind | Output |
|--------|------|--------|
| `map_events` | map | `cryptopunks.types.v1.Events` |
| `db_out` | map | `sf.substreams.sink.database.v1.DatabaseChanges` |

## Tracked Events

- `PunkOffered` — punk listed for sale
- `PunkBought` — punk traded (price, buyer, seller)
- `PunkBidEntered` — bid placed on a punk
- `PunkBidWithdrawn` — bid retracted
- `PunkTransfer` — ownership transfer

## Usage

Run with [substreams-sink-sql](https://github.com/streamingfast/substreams-sink-sql):

```bash
substreams-sink-sql run mainnet.eth.streamingfast.io:443 \
  cryptopunks-substreams-v0.1.0.spkg \
  --schema schema.sql
```

## Origin

This package was generated using the **substreams-convert** skill, with the [Messari CryptoPunks subgraph](https://github.com/messari/subgraphs) as the schema and event-coverage base.
