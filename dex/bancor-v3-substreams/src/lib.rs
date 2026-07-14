mod abi;
mod pb;

use substreams::errors::Error;
use substreams::store::{StoreGet, StoreGetString, StoreNew, StoreSet, StoreSetString};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::bancor_v3::types::v1::{
    Events, BancorNetworkPoolCollectionAdded, BancorNetworkTokensTraded, BntPoolTokensDeposited, BntPoolTokensWithdrawn, BntPoolTotalLiquidityUpdated, NetworkSettingsNetworkFeePpmUpdated, NetworkSettingsWithdrawalFeePpmUpdated, PoolCollectionDefaultTradingFeePpmUpdated, PoolCollectionTokensDeposited, PoolCollectionTokensWithdrawn, PoolCollectionTotalLiquidityUpdated, PoolCollectionTradingFeePpmUpdated, PoolTokenFactoryPoolTokenCreated, StandardRewardsProgramCreated, StandardRewardsProgramEnabled, StandardRewardsProgramTerminated,
};

const FACTORY: [u8; 20] = hex_literal::hex!("eef417e1d5cc832e619ae18d2f140de2999dd4fb");
const POOL_TOKEN_FACTORY: [u8; 20] = hex_literal::hex!("9e912953db31fe933bda43374208e967058d9d5f");
const NETWORK_SETTINGS: [u8; 20] = hex_literal::hex!("83e1814ba31f7ea95d216204bb45fe75ce09b14f");
const STANDARD_REWARDS: [u8; 20] = hex_literal::hex!("b0b958398abb0b5db4ce4d7598fb868f5a00f372");
const BNT_POOL: [u8; 20] = hex_literal::hex!("02651e355d26f3506c1e644ba393fdd9ac95eaca");

fn fmt_addr(addr: &[u8]) -> String {
    format!("0x{}", hex::encode(addr))
}

fn block_timestamp(block: &Block) -> u64 {
    block
        .header
        .as_ref()
        .and_then(|h| h.timestamp.as_ref().map(|t| t.seconds as u64))
        .unwrap_or(0)
}

#[substreams::handlers::store]
pub fn store_pools(block: Block, store: StoreSetString) {
    for trx in block.transactions() {
        for log in trx.receipt().logs() {
            if log.address() == FACTORY.as_slice() {
                if let Some(ev) =
                    abi::bancor_network::events::PoolCollectionAdded::match_and_decode(log)
                {
                    store.set(log.ordinal(), fmt_addr(&ev.pool_collection), &"1".to_string());
                }
            }
        }
    }
}

#[substreams::handlers::map]
pub fn map_events(block: Block, store: StoreGetString) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == FACTORY.as_slice() {
                if let Some(ev) =
                    abi::bancor_network::events::PoolCollectionAdded::match_and_decode(log)
                {
                    events.bancor_network_pool_collection_addeds.push(BancorNetworkPoolCollectionAdded {
                        id: id.clone(),
                        pool_type: ev.pool_type.to_string(),
                        pool_collection: fmt_addr(&ev.pool_collection),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::bancor_network::events::TokensTraded::match_and_decode(log)
                {
                    events.bancor_network_tokens_tradeds.push(BancorNetworkTokensTraded {
                        id: id.clone(),
                        context_id: fmt_addr(&ev.context_id),
                        source_token: fmt_addr(&ev.source_token),
                        target_token: fmt_addr(&ev.target_token),
                        source_amount: ev.source_amount.to_string(),
                        target_amount: ev.target_amount.to_string(),
                        bnt_amount: ev.bnt_amount.to_string(),
                        target_fee_amount: ev.target_fee_amount.to_string(),
                        bnt_fee_amount: ev.bnt_fee_amount.to_string(),
                        trader: fmt_addr(&ev.trader),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if store.get_last(fmt_addr(log.address())).is_some() {
                let pool = fmt_addr(log.address());
                if let Some(ev) =
                    abi::pool_collection::events::TokensDeposited::match_and_decode(log)
                {
                    events.pool_collection_tokens_depositeds.push(PoolCollectionTokensDeposited {
                        id: id.clone(),
                        pool: pool.clone(),
                        context_id: fmt_addr(&ev.context_id),
                        provider: fmt_addr(&ev.provider),
                        token: fmt_addr(&ev.token),
                        token_amount: ev.token_amount.to_string(),
                        pool_token_amount: ev.pool_token_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pool_collection::events::TokensWithdrawn::match_and_decode(log)
                {
                    events.pool_collection_tokens_withdrawns.push(PoolCollectionTokensWithdrawn {
                        id: id.clone(),
                        pool: pool.clone(),
                        context_id: fmt_addr(&ev.context_id),
                        provider: fmt_addr(&ev.provider),
                        token: fmt_addr(&ev.token),
                        token_amount: ev.token_amount.to_string(),
                        pool_token_amount: ev.pool_token_amount.to_string(),
                        external_protection_base_token_amount: ev.external_protection_base_token_amount.to_string(),
                        bnt_amount: ev.bnt_amount.to_string(),
                        withdrawal_fee_amount: ev.withdrawal_fee_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pool_collection::events::TotalLiquidityUpdated::match_and_decode(log)
                {
                    events.pool_collection_total_liquidity_updateds.push(PoolCollectionTotalLiquidityUpdated {
                        id: id.clone(),
                        pool: pool.clone(),
                        context_id: fmt_addr(&ev.context_id),
                        evt_pool: fmt_addr(&ev.pool),
                        liquidity: ev.liquidity.to_string(),
                        staked_balance: ev.staked_balance.to_string(),
                        pool_token_supply: ev.pool_token_supply.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pool_collection::events::DefaultTradingFeePpmUpdated::match_and_decode(log)
                {
                    events.pool_collection_default_trading_fee_ppm_updateds.push(PoolCollectionDefaultTradingFeePpmUpdated {
                        id: id.clone(),
                        pool: pool.clone(),
                        prev_fee_ppm: ev.prev_fee_ppm.to_string(),
                        new_fee_ppm: ev.new_fee_ppm.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pool_collection::events::TradingFeePpmUpdated::match_and_decode(log)
                {
                    events.pool_collection_trading_fee_ppm_updateds.push(PoolCollectionTradingFeePpmUpdated {
                        id: id.clone(),
                        pool: pool.clone(),
                        evt_pool: fmt_addr(&ev.pool),
                        prev_fee_ppm: ev.prev_fee_ppm.to_string(),
                        new_fee_ppm: ev.new_fee_ppm.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == POOL_TOKEN_FACTORY.as_slice() {
                if let Some(ev) =
                    abi::pool_token_factory::events::PoolTokenCreated::match_and_decode(log)
                {
                    events.pool_token_factory_pool_token_createds.push(PoolTokenFactoryPoolTokenCreated {
                        id: id.clone(),
                        pool_token: fmt_addr(&ev.pool_token),
                        token: fmt_addr(&ev.token),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == NETWORK_SETTINGS.as_slice() {
                if let Some(ev) =
                    abi::network_settings::events::NetworkFeePpmUpdated::match_and_decode(log)
                {
                    events.network_settings_network_fee_ppm_updateds.push(NetworkSettingsNetworkFeePpmUpdated {
                        id: id.clone(),
                        prev_fee_ppm: ev.prev_fee_ppm.to_string(),
                        new_fee_ppm: ev.new_fee_ppm.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::network_settings::events::WithdrawalFeePpmUpdated::match_and_decode(log)
                {
                    events.network_settings_withdrawal_fee_ppm_updateds.push(NetworkSettingsWithdrawalFeePpmUpdated {
                        id: id.clone(),
                        prev_fee_ppm: ev.prev_fee_ppm.to_string(),
                        new_fee_ppm: ev.new_fee_ppm.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == STANDARD_REWARDS.as_slice() {
                if let Some(ev) =
                    abi::standard_rewards::events::ProgramCreated::match_and_decode(log)
                {
                    events.standard_rewards_program_createds.push(StandardRewardsProgramCreated {
                        id: id.clone(),
                        pool: fmt_addr(&ev.pool),
                        program_id: ev.program_id.to_string(),
                        rewards_token: fmt_addr(&ev.rewards_token),
                        total_rewards: ev.total_rewards.to_string(),
                        start_time: ev.start_time.to_string(),
                        end_time: ev.end_time.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::standard_rewards::events::ProgramTerminated::match_and_decode(log)
                {
                    events.standard_rewards_program_terminateds.push(StandardRewardsProgramTerminated {
                        id: id.clone(),
                        pool: fmt_addr(&ev.pool),
                        program_id: ev.program_id.to_string(),
                        end_time: ev.end_time.to_string(),
                        remaining_rewards: ev.remaining_rewards.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::standard_rewards::events::ProgramEnabled::match_and_decode(log)
                {
                    events.standard_rewards_program_enableds.push(StandardRewardsProgramEnabled {
                        id: id.clone(),
                        pool: fmt_addr(&ev.pool),
                        program_id: ev.program_id.to_string(),
                        status: ev.status.to_string(),
                        remaining_rewards: ev.remaining_rewards.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == BNT_POOL.as_slice() {
                if let Some(ev) =
                    abi::bnt_pool::events::TokensDeposited::match_and_decode(log)
                {
                    events.bnt_pool_tokens_depositeds.push(BntPoolTokensDeposited {
                        id: id.clone(),
                        context_id: fmt_addr(&ev.context_id),
                        provider: fmt_addr(&ev.provider),
                        bnt_amount: ev.bnt_amount.to_string(),
                        pool_token_amount: ev.pool_token_amount.to_string(),
                        vbnt_amount: ev.vbnt_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::bnt_pool::events::TokensWithdrawn::match_and_decode(log)
                {
                    events.bnt_pool_tokens_withdrawns.push(BntPoolTokensWithdrawn {
                        id: id.clone(),
                        context_id: fmt_addr(&ev.context_id),
                        provider: fmt_addr(&ev.provider),
                        bnt_amount: ev.bnt_amount.to_string(),
                        pool_token_amount: ev.pool_token_amount.to_string(),
                        vbnt_amount: ev.vbnt_amount.to_string(),
                        withdrawal_fee_amount: ev.withdrawal_fee_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::bnt_pool::events::TotalLiquidityUpdated::match_and_decode(log)
                {
                    events.bnt_pool_total_liquidity_updateds.push(BntPoolTotalLiquidityUpdated {
                        id: id.clone(),
                        context_id: fmt_addr(&ev.context_id),
                        liquidity: ev.liquidity.to_string(),
                        staked_balance: ev.staked_balance.to_string(),
                        pool_token_supply: ev.pool_token_supply.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.bancor_network_pool_collection_addeds {
        tables
            .create_row("bancor_network_pool_collection_added", &e.id)
            .set("pool_type", &e.pool_type)
            .set("pool_collection", &e.pool_collection)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.bancor_network_tokens_tradeds {
        tables
            .create_row("bancor_network_tokens_traded", &e.id)
            .set("context_id", &e.context_id)
            .set("source_token", &e.source_token)
            .set("target_token", &e.target_token)
            .set("source_amount", &e.source_amount)
            .set("target_amount", &e.target_amount)
            .set("bnt_amount", &e.bnt_amount)
            .set("target_fee_amount", &e.target_fee_amount)
            .set("bnt_fee_amount", &e.bnt_fee_amount)
            .set("trader", &e.trader)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pool_collection_tokens_depositeds {
        tables
            .create_row("pool_collection_tokens_deposited", &e.id)
            .set("pool", &e.pool)
            .set("context_id", &e.context_id)
            .set("provider", &e.provider)
            .set("token", &e.token)
            .set("token_amount", &e.token_amount)
            .set("pool_token_amount", &e.pool_token_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pool_collection_tokens_withdrawns {
        tables
            .create_row("pool_collection_tokens_withdrawn", &e.id)
            .set("pool", &e.pool)
            .set("context_id", &e.context_id)
            .set("provider", &e.provider)
            .set("token", &e.token)
            .set("token_amount", &e.token_amount)
            .set("pool_token_amount", &e.pool_token_amount)
            .set("external_protection_base_token_amount", &e.external_protection_base_token_amount)
            .set("bnt_amount", &e.bnt_amount)
            .set("withdrawal_fee_amount", &e.withdrawal_fee_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pool_collection_total_liquidity_updateds {
        tables
            .create_row("pool_collection_total_liquidity_updated", &e.id)
            .set("pool", &e.pool)
            .set("context_id", &e.context_id)
            .set("evt_pool", &e.evt_pool)
            .set("liquidity", &e.liquidity)
            .set("staked_balance", &e.staked_balance)
            .set("pool_token_supply", &e.pool_token_supply)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pool_collection_default_trading_fee_ppm_updateds {
        tables
            .create_row("pool_collection_default_trading_fee_ppm_updated", &e.id)
            .set("pool", &e.pool)
            .set("prev_fee_ppm", &e.prev_fee_ppm)
            .set("new_fee_ppm", &e.new_fee_ppm)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pool_collection_trading_fee_ppm_updateds {
        tables
            .create_row("pool_collection_trading_fee_ppm_updated", &e.id)
            .set("pool", &e.pool)
            .set("evt_pool", &e.evt_pool)
            .set("prev_fee_ppm", &e.prev_fee_ppm)
            .set("new_fee_ppm", &e.new_fee_ppm)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pool_token_factory_pool_token_createds {
        tables
            .create_row("pool_token_factory_pool_token_created", &e.id)
            .set("pool_token", &e.pool_token)
            .set("token", &e.token)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.network_settings_network_fee_ppm_updateds {
        tables
            .create_row("network_settings_network_fee_ppm_updated", &e.id)
            .set("prev_fee_ppm", &e.prev_fee_ppm)
            .set("new_fee_ppm", &e.new_fee_ppm)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.network_settings_withdrawal_fee_ppm_updateds {
        tables
            .create_row("network_settings_withdrawal_fee_ppm_updated", &e.id)
            .set("prev_fee_ppm", &e.prev_fee_ppm)
            .set("new_fee_ppm", &e.new_fee_ppm)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.standard_rewards_program_createds {
        tables
            .create_row("standard_rewards_program_created", &e.id)
            .set("pool", &e.pool)
            .set("program_id", &e.program_id)
            .set("rewards_token", &e.rewards_token)
            .set("total_rewards", &e.total_rewards)
            .set("start_time", &e.start_time)
            .set("end_time", &e.end_time)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.standard_rewards_program_terminateds {
        tables
            .create_row("standard_rewards_program_terminated", &e.id)
            .set("pool", &e.pool)
            .set("program_id", &e.program_id)
            .set("end_time", &e.end_time)
            .set("remaining_rewards", &e.remaining_rewards)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.standard_rewards_program_enableds {
        tables
            .create_row("standard_rewards_program_enabled", &e.id)
            .set("pool", &e.pool)
            .set("program_id", &e.program_id)
            .set("status", &e.status)
            .set("remaining_rewards", &e.remaining_rewards)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.bnt_pool_tokens_depositeds {
        tables
            .create_row("bnt_pool_tokens_deposited", &e.id)
            .set("context_id", &e.context_id)
            .set("provider", &e.provider)
            .set("bnt_amount", &e.bnt_amount)
            .set("pool_token_amount", &e.pool_token_amount)
            .set("vbnt_amount", &e.vbnt_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.bnt_pool_tokens_withdrawns {
        tables
            .create_row("bnt_pool_tokens_withdrawn", &e.id)
            .set("context_id", &e.context_id)
            .set("provider", &e.provider)
            .set("bnt_amount", &e.bnt_amount)
            .set("pool_token_amount", &e.pool_token_amount)
            .set("vbnt_amount", &e.vbnt_amount)
            .set("withdrawal_fee_amount", &e.withdrawal_fee_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.bnt_pool_total_liquidity_updateds {
        tables
            .create_row("bnt_pool_total_liquidity_updated", &e.id)
            .set("context_id", &e.context_id)
            .set("liquidity", &e.liquidity)
            .set("staked_balance", &e.staked_balance)
            .set("pool_token_supply", &e.pool_token_supply)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
