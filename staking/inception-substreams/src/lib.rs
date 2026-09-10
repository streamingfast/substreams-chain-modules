mod abi;
// buffa emits view re-exports for every message; most modules use only the owned type.
#[allow(unused_imports)]
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::BlockLazyView;
use substreams_ethereum::Event;

use crate::pb::inception::types::v1::{Events, InethTransfer};

const INETH: [u8; 20] = hex_literal::hex!("f073bac22dab7faf4a3dd6c6189a70d54110525c");

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

        let Some(receipt) = trx.receipt()? else {
            continue;
        };
        for log in receipt.logs.iter() {
            let log = log?;
            let id = format!("{}-{}", tx_hash, log.index);

            if log.address == INETH.as_slice() {
                if let Some(ev) = abi::ineth::events::Transfer::match_and_decode(&log) {
                    events.ineth_transfers.push(InethTransfer {
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

    for e in events.ineth_transfers {
        tables
            .create_row("ineth_transfer", &e.id)
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
