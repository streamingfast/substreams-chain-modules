mod abi;
// buffa emits view re-exports for every message; most modules use only the owned type.
#[allow(unused_imports)]
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::BlockLazyView;
use substreams_ethereum::Event;

use crate::pb::creth2::types::v1::{CreamTransfer, CrethTransfer, Events};

const CREAM: [u8; 20] = hex_literal::hex!("2ba592f78db6436527729929aaf6c908497cb200");
const CRETH: [u8; 20] = hex_literal::hex!("49d72e3973900a195a155a46441f0c08179fdb64");

fn fmt_addr(addr: &[u8]) -> String {
    format!("0x{}", hex::encode(addr))
}

fn block_timestamp(block: &BlockLazyView<'_>) -> u64 {
    block
        .header
        .get()
        .ok()
        .flatten()
        .map(|h| h.timestamp.as_option().map(|t| t.seconds).unwrap_or(0))
        .unwrap_or(0) as u64
}

#[substreams::handlers::map]
pub fn map_events(block: &BlockLazyView<'_>) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for lc in trx.logs_with_calls()? {
            let log = &lc.log;
            let id = format!("{}-{}", tx_hash, log.index);

            if log.address == CREAM {
                if let Some(ev) = abi::cream::events::Transfer::match_and_decode(&log) {
                    events.cream_transfers.push(CreamTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == CRETH {
                if let Some(ev) = abi::creth::events::Transfer::match_and_decode(&log) {
                    events.creth_transfers.push(CrethTransfer {
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
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.cream_transfers {
        tables
            .create_row("cream_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.creth_transfers {
        tables
            .create_row("creth_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
