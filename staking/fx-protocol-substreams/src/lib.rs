mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::fx_protocol::types::v1::{
    Events, PoolManagerHarvest,
};

const POOL_MANAGER: [u8; 20] = hex_literal::hex!("250893ca4ba5d05626c785e8da758026928fcd24");

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
            let id = format!("{}-{}", tx_hash, log.index);

            if log.address == POOL_MANAGER {
                if let Some(ev) =
                    abi::pool_manager::events::Harvest::match_and_decode(log)
                {
                    events.pool_manager_harvests.push(PoolManagerHarvest {
                        id: id.clone(),
                        caller: fmt_addr(&ev.caller),
                        pool: fmt_addr(&ev.pool),
                        amount_rewards: ev.amount_rewards.to_string(),
                        amount_funding: ev.amount_funding.to_string(),
                        performance_fee: ev.performance_fee.to_string(),
                        harvest_bounty: ev.harvest_bounty.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
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

    for e in events.pool_manager_harvests {
        tables
            .create_row("pool_manager_harvest", &e.id)
            .set("caller", &e.caller)
            .set("pool", &e.pool)
            .set("amount_rewards", &e.amount_rewards)
            .set("amount_funding", &e.amount_funding)
            .set("performance_fee", &e.performance_fee)
            .set("harvest_bounty", &e.harvest_bounty)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
