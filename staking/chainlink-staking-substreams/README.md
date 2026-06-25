# chainlink-staking-substreams

Substreams package indexing Chainlink LINK staking events across CommunityStakingPool, OperatorStakingPool, RewardVault, and legacy Staking V1 on Ethereum mainnet. Outputs `DatabaseChanges` for SQL sink consumption.

- **Network**: Ethereum Mainnet
- **Start block**: 16,083,969 (Chainlink Staking v1 deployment)
- **Package**: `chainlink-staking-substreams-v0.1.0.spkg`

## Modules

| Module | Kind | Output |
|--------|------|--------|
| `map_events` | map | `chainlink_staking.types.v1.Events` |
| `store_reward_vaults` | store | tracks deployed RewardVault addresses |
| `map_reward_vault_events` | map | `chainlink_staking.types.v1.Events` |
| `db_out` | map | `sf.substreams.sink.database.v1.DatabaseChanges` |

## Tracked Events

**Staking V1 (legacy)**
- `Staked` — LINK staked
- `Unstaked` — LINK unstaked
- `RewardInitialized` / `RewardReset` — reward configuration

**Staking V2 — CommunityStakingPool & OperatorStakingPool**
- `Staked` / `Unstaked` / `Unbonding` / `UnbondingPeriodStarted`
- `PoolOpened` / `PoolClosed` / `PoolSizeIncreased`
- `MaxPrincipalAmountUpdated`

**RewardVault**
- `RewardAdded` — LINK reward deposited
- `RewardClaimed` — staker claims reward
- `VaultOpened` / `VaultClosed` / `VaultVersionUpgraded`
- `DelegationRateDenominatorUpdated` / `MultiplierDurationUpdated`

## Usage

Run with [substreams-sink-sql](https://github.com/streamingfast/substreams-sink-sql):

```bash
substreams-sink-sql run mainnet.eth.streamingfast.io:443 \
  chainlink-staking-substreams-v0.1.0.spkg \
  --schema schema.sql
```

## Origin

This package was generated using the **substreams-convert** skill, with the [Messari Chainlink subgraph](https://github.com/messari/subgraphs) as the schema and event-coverage base. Covers both Staking V1 and V2 (including RewardVault), categorized under Staking rather than Oracles.
