mod abi;
mod pb;

use substreams::errors::Error;
use substreams::store::{StoreGet, StoreGetString, StoreNew, StoreSetIfNotExists, StoreSetIfNotExistsString};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::chainlink_staking::types::v1::{
    Events, RewardUpdate, RewardVaultSet, StakeEvent, StakeEventV1,
};

// CommunityStakingPool on Ethereum mainnet — deployed block 18572190
const COMMUNITY_STAKING_POOL: [u8; 20] =
    hex_literal::hex!("bc10f2e862ed4502144c7d632a3459f49dfcdb5e");
// OperatorStakingPool on Ethereum mainnet — deployed block 18572190
const OPERATOR_STAKING_POOL: [u8; 20] =
    hex_literal::hex!("a1d76a7ca72128541e9fcacafbda3a92ef94fdc5");
// Legacy Staking V1 — deployed block 16083969
const STAKING_V1: [u8; 20] = hex_literal::hex!("3feb1e09b4bb0e7f0387cee092a52e85797ab889");

fn block_timestamp(block: &Block) -> u64 {
    block
        .header
        .as_ref()
        .and_then(|h| h.timestamp.as_ref().map(|t| t.seconds as u64))
        .unwrap_or(0)
}

fn pool_name(addr: &[u8; 20]) -> &'static str {
    if addr == &COMMUNITY_STAKING_POOL {
        "community"
    } else {
        "operator"
    }
}

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for (log, _call) in trx.logs_with_calls() {
            let addr = &log.address;

            if addr == &COMMUNITY_STAKING_POOL || addr == &OPERATOR_STAKING_POOL {
                let pool = pool_name(addr.as_slice().try_into().unwrap_or(&[0u8; 20]));

                if let Some(ev) = abi::staking_pool::events::Staked::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.stake_events.push(StakeEvent {
                        id,
                        pool: pool.to_string(),
                        staker: format!("0x{}", hex::encode(ev.staker)),
                        event_type: "staked".to_string(),
                        amount: ev.amount.to_string(),
                        new_stake: ev.new_stake.to_string(),
                        new_total_principal: ev.new_total_principal.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) = abi::staking_pool::events::Unstaked::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.stake_events.push(StakeEvent {
                        id,
                        pool: pool.to_string(),
                        staker: format!("0x{}", hex::encode(ev.staker)),
                        event_type: "unstaked".to_string(),
                        amount: ev.amount.to_string(),
                        new_stake: ev.new_stake.to_string(),
                        new_total_principal: ev.new_total_principal.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) = abi::staking_pool::events::RewardVaultSet::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.reward_vault_sets.push(RewardVaultSet {
                        id,
                        pool: pool.to_string(),
                        old_reward_vault: format!("0x{}", hex::encode(ev.old_reward_vault)),
                        new_reward_vault: format!("0x{}", hex::encode(ev.new_reward_vault)),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if addr == &STAKING_V1 {
                if let Some(ev) = abi::staking_v1::events::Staked::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.stake_events_v1.push(StakeEventV1 {
                        id,
                        staker: format!("0x{}", hex::encode(ev.staker)),
                        event_type: "staked".to_string(),
                        new_stake: ev.new_stake.to_string(),
                        total_stake: ev.total_stake.to_string(),
                        principal: String::new(),
                        base_reward: String::new(),
                        delegation_reward: String::new(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) = abi::staking_v1::events::Unstaked::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.stake_events_v1.push(StakeEventV1 {
                        id,
                        staker: format!("0x{}", hex::encode(ev.staker)),
                        event_type: "unstaked".to_string(),
                        new_stake: String::new(),
                        total_stake: String::new(),
                        principal: ev.principal.to_string(),
                        base_reward: ev.base_reward.to_string(),
                        delegation_reward: ev.delegation_reward.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::store]
pub fn store_reward_vaults(events: Events, store: StoreSetIfNotExistsString) {
    for rv_set in events.reward_vault_sets {
        store.set_if_not_exists(0, &rv_set.new_reward_vault, &rv_set.pool);
    }
}

#[substreams::handlers::map]
pub fn map_reward_vault_events(block: Block, store: StoreGetString) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for (log, _call) in trx.logs_with_calls() {
            let vault_addr = format!("0x{}", hex::encode(&log.address));

            if store.get_last(&vault_addr).is_none() {
                continue;
            }

            let pool_type = store.get_last(&vault_addr).unwrap_or_default();

            if let Some(ev) =
                abi::reward_vault::events::CommunityPoolRewardUpdated::match_and_decode(log)
            {
                let id = format!("{}-{}", tx_hash, log.index);
                events.reward_updates.push(RewardUpdate {
                    id,
                    pool_type: pool_type.clone(),
                    base_reward_per_token: ev.base_reward_per_token.to_string(),
                    delegated_reward_per_token: String::new(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::reward_vault::events::OperatorPoolRewardUpdated::match_and_decode(log)
            {
                let id = format!("{}-{}", tx_hash, log.index);
                events.reward_updates.push(RewardUpdate {
                    id,
                    pool_type: pool_type.clone(),
                    base_reward_per_token: ev.base_reward_per_token.to_string(),
                    delegated_reward_per_token: ev.delegated_reward_per_token.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events, vault_events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for ev in events.stake_events.iter().chain(vault_events.stake_events.iter()) {
        tables
            .create_row("stake_events", &ev.id)
            .set("pool", &ev.pool)
            .set("staker", &ev.staker)
            .set("event_type", &ev.event_type)
            .set("amount", &ev.amount)
            .set("new_stake", &ev.new_stake)
            .set("new_total_principal", &ev.new_total_principal)
            .set("tx_hash", &ev.tx_hash)
            .set("log_index", ev.log_index as i64)
            .set("block_num", ev.block_num as i64)
            .set("timestamp", ev.timestamp as i64);
    }

    for ev in events.stake_events_v1.iter().chain(vault_events.stake_events_v1.iter()) {
        tables
            .create_row("stake_events_v1", &ev.id)
            .set("staker", &ev.staker)
            .set("event_type", &ev.event_type)
            .set("new_stake", &ev.new_stake)
            .set("total_stake", &ev.total_stake)
            .set("principal", &ev.principal)
            .set("base_reward", &ev.base_reward)
            .set("delegation_reward", &ev.delegation_reward)
            .set("tx_hash", &ev.tx_hash)
            .set("log_index", ev.log_index as i64)
            .set("block_num", ev.block_num as i64)
            .set("timestamp", ev.timestamp as i64);
    }

    for ev in events.reward_updates.iter().chain(vault_events.reward_updates.iter()) {
        tables
            .create_row("reward_updates", &ev.id)
            .set("pool_type", &ev.pool_type)
            .set("base_reward_per_token", &ev.base_reward_per_token)
            .set("delegated_reward_per_token", &ev.delegated_reward_per_token)
            .set("tx_hash", &ev.tx_hash)
            .set("log_index", ev.log_index as i64)
            .set("block_num", ev.block_num as i64)
            .set("timestamp", ev.timestamp as i64);
    }

    for ev in events.reward_vault_sets.iter().chain(vault_events.reward_vault_sets.iter()) {
        tables
            .create_row("reward_vault_sets", &ev.id)
            .set("pool", &ev.pool)
            .set("old_reward_vault", &ev.old_reward_vault)
            .set("new_reward_vault", &ev.new_reward_vault)
            .set("tx_hash", &ev.tx_hash)
            .set("log_index", ev.log_index as i64)
            .set("block_num", ev.block_num as i64)
            .set("timestamp", ev.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
