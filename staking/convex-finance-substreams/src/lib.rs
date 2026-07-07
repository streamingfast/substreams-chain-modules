mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::convex::types::v1::{Deposited, Events, Withdrawn};

const BOOSTER: [u8; 20] = hex_literal::hex!("f403c135812408bfbe8713b5a23a04b3d48aae31");

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
            if log.address != BOOSTER {
                continue;
            }

            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) = abi::booster::events::Deposited::match_and_decode(log) {
                events.deposited.push(Deposited {
                    id,
                    user: fmt_addr(&ev.user),
                    pool_id: ev.poolid.to_u64(),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::booster::events::Withdrawn::match_and_decode(log) {
                events.withdrawn.push(Withdrawn {
                    id,
                    user: fmt_addr(&ev.user),
                    pool_id: ev.poolid.to_u64(),
                    amount: ev.amount.to_string(),
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

    for e in events.deposited {
        tables
            .create_row("deposited", &e.id)
            .set("user", &e.user)
            .set("pool_id", e.pool_id as i64)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawn {
        tables
            .create_row("withdrawn", &e.id)
            .set("user", &e.user)
            .set("pool_id", e.pool_id as i64)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
