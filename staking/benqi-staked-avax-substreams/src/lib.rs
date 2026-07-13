mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::benqi_staked_avax::types::v1::{
    Events, SavaxAccrueRewards, SavaxOldImplementationAccrueRewards, SavaxRedeem, SavaxSubmitted,
};

const SAVAX: [u8; 20] = hex_literal::hex!("2b2c81e08f1af8835a78bb2a90ae924ace0ea4be");
const SAVAX_OLD_IMPLEMENTATION: [u8; 20] = hex_literal::hex!("2b2c81e08f1af8835a78bb2a90ae924ace0ea4be");

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

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == SAVAX.as_slice() {
                if let Some(ev) =
                    abi::savax::events::Submitted::match_and_decode(log)
                {
                    events.savax_submitteds.push(SavaxSubmitted {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        avax_amount: ev.avax_amount.to_string(),
                        share_amount: ev.share_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::savax::events::Redeem::match_and_decode(log)
                {
                    events.savax_redeems.push(SavaxRedeem {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        unlock_requested_at: ev.unlock_requested_at.to_string(),
                        share_amount: ev.share_amount.to_string(),
                        avax_amount: ev.avax_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::savax::events::AccrueRewards::match_and_decode(log)
                {
                    events.savax_accrue_rewardss.push(SavaxAccrueRewards {
                        id: id.clone(),
                        user_reward_amount: ev.user_reward_amount.to_string(),
                        protocol_reward_amount: ev.protocol_reward_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == SAVAX_OLD_IMPLEMENTATION.as_slice() {
                if let Some(ev) =
                    abi::savax_old_implementation::events::AccrueRewards::match_and_decode(log)
                {
                    events.savax_old_implementation_accrue_rewardss.push(SavaxOldImplementationAccrueRewards {
                        id: id.clone(),
                        value: ev.value.to_string(),
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

    for e in events.savax_submitteds {
        tables
            .create_row("savax_submitted", &e.id)
            .set("user", &e.user)
            .set("avax_amount", &e.avax_amount)
            .set("share_amount", &e.share_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.savax_redeems {
        tables
            .create_row("savax_redeem", &e.id)
            .set("user", &e.user)
            .set("unlock_requested_at", &e.unlock_requested_at)
            .set("share_amount", &e.share_amount)
            .set("avax_amount", &e.avax_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.savax_accrue_rewardss {
        tables
            .create_row("savax_accrue_rewards", &e.id)
            .set("user_reward_amount", &e.user_reward_amount)
            .set("protocol_reward_amount", &e.protocol_reward_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.savax_old_implementation_accrue_rewardss {
        tables
            .create_row("savax_old_implementation_accrue_rewards", &e.id)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
