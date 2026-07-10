mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::stader::types::v1::{
    Deposit, Events, ExchangeRateUpdate, OperatorRewardsClaimed, ProtocolEthRewardsTransferred,
    UserEthRewardsTransferred,
};

const STAKING_POOL_MANAGER: [u8; 20] =
    hex_literal::hex!("cf5ea1b38380f6af39068375516daf40ed70d299");
const STADER_ORACLE: [u8; 20] = hex_literal::hex!("f64bae65f6f2a5277571143a24faafdfc0c2a737");
const SOCIALIZING_POOL_PERMISSIONED: [u8; 20] =
    hex_literal::hex!("9d4c3166c59412cedbe7d901f5fde41903a1d6fc");
const SOCIALIZING_POOL_PERMISSIONLESS: [u8; 20] =
    hex_literal::hex!("1de458031bfbe5689ded5a8b9ed57e1e79eab2a4");

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

fn is_socializing_pool(addr: &[u8]) -> bool {
    addr == SOCIALIZING_POOL_PERMISSIONED || addr == SOCIALIZING_POOL_PERMISSIONLESS
}

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for (log, _call) in trx.logs_with_calls() {
            let id = format!("{}-{}", tx_hash, log.index);

            if log.address == STAKING_POOL_MANAGER {
                if let Some(ev) =
                    abi::staking_pool_manager::events::Deposited::match_and_decode(log)
                {
                    events.deposits.push(Deposit {
                        id,
                        caller: fmt_addr(&ev.caller),
                        owner: fmt_addr(&ev.owner),
                        assets: ev.assets.to_string(),
                        shares: ev.shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == STADER_ORACLE {
                if let Some(ev) =
                    abi::stader_oracle::events::ExchangeRateUpdated::match_and_decode(log)
                {
                    events.exchange_rate_updates.push(ExchangeRateUpdate {
                        id,
                        reporting_block: ev.block.to_string(),
                        total_eth: ev.total_eth.to_string(),
                        ethx_supply: ev.ethx_supply.to_string(),
                        time: ev.time.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if is_socializing_pool(&log.address) {
                let pool = fmt_addr(&log.address);

                if let Some(ev) =
                    abi::socializing_pool::events::ProtocolEthRewardsTransferred::match_and_decode(
                        log,
                    )
                {
                    events.protocol_eth_rewards.push(ProtocolEthRewardsTransferred {
                        id,
                        pool,
                        eth_rewards: ev.eth_rewards.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::socializing_pool::events::UserEthRewardsTransferred::match_and_decode(log)
                {
                    events.user_eth_rewards.push(UserEthRewardsTransferred {
                        id,
                        pool,
                        eth_rewards: ev.eth_rewards.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::socializing_pool::events::OperatorRewardsClaimed::match_and_decode(log)
                {
                    events.operator_rewards_claimed.push(OperatorRewardsClaimed {
                        id,
                        pool,
                        recipient: fmt_addr(&ev.recipient),
                        eth_rewards: ev.eth_rewards.to_string(),
                        sd_rewards: ev.sd_rewards.to_string(),
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

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.deposits {
        tables
            .create_row("deposits", &e.id)
            .set("caller", &e.caller)
            .set("owner", &e.owner)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.exchange_rate_updates {
        tables
            .create_row("exchange_rate_updates", &e.id)
            .set("reporting_block", &e.reporting_block)
            .set("total_eth", &e.total_eth)
            .set("ethx_supply", &e.ethx_supply)
            .set("time", &e.time)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.protocol_eth_rewards {
        tables
            .create_row("protocol_eth_rewards", &e.id)
            .set("pool", &e.pool)
            .set("eth_rewards", &e.eth_rewards)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.user_eth_rewards {
        tables
            .create_row("user_eth_rewards", &e.id)
            .set("pool", &e.pool)
            .set("eth_rewards", &e.eth_rewards)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.operator_rewards_claimed {
        tables
            .create_row("operator_rewards_claimed", &e.id)
            .set("pool", &e.pool)
            .set("recipient", &e.recipient)
            .set("eth_rewards", &e.eth_rewards)
            .set("sd_rewards", &e.sd_rewards)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
