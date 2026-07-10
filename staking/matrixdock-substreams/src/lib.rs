mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::matrixdock::types::v1::{
    Events, StbtInterestsDistributed, StbtTransfer,
};

const STBT: [u8; 20] = hex_literal::hex!("530824da86689c9c17cdc2871ff29b058345b44a");

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

            if log.address == STBT {
                if let Some(ev) =
                    abi::stbt::events::Transfer::match_and_decode(log)
                {
                    events.stbt_transfers.push(StbtTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        value: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::stbt::events::InterestsDistributed::match_and_decode(log)
                {
                    events.stbt_interests_distributeds.push(StbtInterestsDistributed {
                        id: id.clone(),
                        interest: ev.interest.to_string(),
                        new_total_supply: ev.new_total_supply.to_string(),
                        interest_from_time: ev.interest_from_time.to_string(),
                        interest_to_time: ev.interest_to_time.to_string(),
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

    for e in events.stbt_transfers {
        tables
            .create_row("stbt_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stbt_interests_distributeds {
        tables
            .create_row("stbt_interests_distributed", &e.id)
            .set("interest", &e.interest)
            .set("new_total_supply", &e.new_total_supply)
            .set("interest_from_time", &e.interest_from_time)
            .set("interest_to_time", &e.interest_to_time)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
