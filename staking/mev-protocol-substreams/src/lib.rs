mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::mev_protocol::types::v1::{
    Events, MevethDeposit, MevethRewards, MevethWithdraw,
};

const MEVETH: [u8; 20] = hex_literal::hex!("24ae2da0f361aa4be46b48eb19c91e02c5e4f27e");

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

            if log.address() == MEVETH.as_slice() {
                if let Some(ev) =
                    abi::meveth::events::Deposit::match_and_decode(log)
                {
                    events.meveth_deposits.push(MevethDeposit {
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
                    abi::meveth::events::Withdraw::match_and_decode(log)
                {
                    events.meveth_withdraws.push(MevethWithdraw {
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
                    abi::meveth::events::Rewards::match_and_decode(log)
                {
                    events.meveth_rewardss.push(MevethRewards {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
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

    for e in events.meveth_deposits {
        tables
            .create_row("meveth_deposit", &e.id)
            .set("caller", &e.caller)
            .set("owner", &e.owner)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.meveth_withdraws {
        tables
            .create_row("meveth_withdraw", &e.id)
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

    for e in events.meveth_rewardss {
        tables
            .create_row("meveth_rewards", &e.id)
            .set("sender", &e.sender)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
