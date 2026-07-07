mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::frax::types::v1::{Deposit, Events, NewRewardsCycle, Withdraw};

const SFRXETH: [u8; 20] = hex_literal::hex!("ac3e018457b222d93114458476f3e3416abbe38f");

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

        for (log, _call) in trx.logs_with_calls() {
            if log.address != SFRXETH {
                continue;
            }

            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) = abi::sfrxeth::events::Deposit::match_and_decode(log) {
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

            if let Some(ev) = abi::sfrxeth::events::Withdraw::match_and_decode(log) {
                events.withdrawals.push(Withdraw {
                    id,
                    caller: fmt_addr(&ev.caller),
                    receiver: fmt_addr(&ev.receiver),
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

            if let Some(ev) = abi::sfrxeth::events::NewRewardsCycle::match_and_decode(log) {
                events.rewards_cycles.push(NewRewardsCycle {
                    id,
                    cycle_end: ev.cycle_end.to_u64() as u32,
                    reward_amount: ev.reward_amount.to_string(),
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

    for e in events.withdrawals {
        tables
            .create_row("withdrawals", &e.id)
            .set("caller", &e.caller)
            .set("receiver", &e.receiver)
            .set("owner", &e.owner)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rewards_cycles {
        tables
            .create_row("rewards_cycles", &e.id)
            .set("cycle_end", e.cycle_end as i64)
            .set("reward_amount", &e.reward_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
