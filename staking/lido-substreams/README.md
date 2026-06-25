# lido-substreams

Substreams package indexing Lido stETH deposits, transfers, oracle reports, and withdrawal events on Ethereum mainnet. Outputs `DatabaseChanges` for SQL sink consumption.

- **Network**: Ethereum Mainnet
- **Start block**: 11,473,216 (Lido contract deployment)
- **Package**: `lido-substreams-v0.1.0.spkg`

## Modules

| Module | Kind | Output |
|--------|------|--------|
| `map_events` | map | `lido.types.v1.Events` |
| `db_out` | map | `sf.substreams.sink.database.v1.DatabaseChanges` |

## Tracked Events

- `Submitted` — ETH deposited, stETH minted
- `Transfer` — stETH transferred between accounts
- `TransferShares` — underlying share transfers
- `TokenRebased` — oracle post-rebase report (total shares, ETH, APR)
- `WithdrawalRequested` — withdrawal NFT minted
- `WithdrawalsFinalized` — batch withdrawal finalized
- `Approval` — stETH approval

## Usage

Run with [substreams-sink-sql](https://github.com/streamingfast/substreams-sink-sql):

```bash
substreams-sink-sql run mainnet.eth.streamingfast.io:443 \
  lido-substreams-v0.1.0.spkg \
  --schema schema.sql
```

## Origin

This package was generated using the **substreams-convert** skill, with the [Messari Lido subgraph](https://github.com/messari/subgraphs) as the schema and event-coverage base. It covers a richer event set than the existing `lido-steth` package, adding oracle rebase reports and withdrawal queue tracking.
