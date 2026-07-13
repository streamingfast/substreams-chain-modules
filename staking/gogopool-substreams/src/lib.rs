mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::gogopool::types::v1::{
    Events, GgavaxDeposit, GgavaxDepositedFromStaking, GgavaxNewRewardsCycle, GgavaxWithdraw, GgavaxWithdrawnForStaking,
};

const GGAVAX: [u8; 20] = hex_literal::hex!("a25eaf2906fa1a3a13edac9b9657108af7b703e3");

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

            if log.address() == GGAVAX.as_slice() {
                if let Some(ev) =
                    abi::ggavax::events::Deposit::match_and_decode(log)
                {
                    events.ggavax_deposits.push(GgavaxDeposit {
                        id: id.clone(),
                        caller: fmt_addr(&ev.caller),
                        owner: fmt_addr(&ev.owner),
                        assets: ev.assets.to_string(),
                        shares: ev.shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ggavax::events::DepositedFromStaking::match_and_decode(log)
                {
                    events.ggavax_deposited_from_stakings.push(GgavaxDepositedFromStaking {
                        id: id.clone(),
                        caller: fmt_addr(&ev.caller),
                        base_amt: ev.base_amt.to_string(),
                        rewards_amt: ev.rewards_amt.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ggavax::events::Withdraw::match_and_decode(log)
                {
                    events.ggavax_withdraws.push(GgavaxWithdraw {
                        id: id.clone(),
                        caller: fmt_addr(&ev.caller),
                        receiver: fmt_addr(&ev.receiver),
                        owner: fmt_addr(&ev.owner),
                        assets: ev.assets.to_string(),
                        shares: ev.shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ggavax::events::WithdrawnForStaking::match_and_decode(log)
                {
                    events.ggavax_withdrawn_for_stakings.push(GgavaxWithdrawnForStaking {
                        id: id.clone(),
                        caller: fmt_addr(&ev.caller),
                        assets: ev.assets.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ggavax::events::NewRewardsCycle::match_and_decode(log)
                {
                    events.ggavax_new_rewards_cycles.push(GgavaxNewRewardsCycle {
                        id: id.clone(),
                        cycle_end: ev.cycle_end.to_string(),
                        rewards_amt: ev.rewards_amt.to_string(),
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

    for e in events.ggavax_deposits {
        tables
            .create_row("ggavax_deposit", &e.id)
            .set("caller", &e.caller)
            .set("owner", &e.owner)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ggavax_deposited_from_stakings {
        tables
            .create_row("ggavax_deposited_from_staking", &e.id)
            .set("caller", &e.caller)
            .set("base_amt", &e.base_amt)
            .set("rewards_amt", &e.rewards_amt)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ggavax_withdraws {
        tables
            .create_row("ggavax_withdraw", &e.id)
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

    for e in events.ggavax_withdrawn_for_stakings {
        tables
            .create_row("ggavax_withdrawn_for_staking", &e.id)
            .set("caller", &e.caller)
            .set("assets", &e.assets)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ggavax_new_rewards_cycles {
        tables
            .create_row("ggavax_new_rewards_cycle", &e.id)
            .set("cycle_end", &e.cycle_end)
            .set("rewards_amt", &e.rewards_amt)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
