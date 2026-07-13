mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::usual::types::v1::{
    Events, UsycFeeProcessed, UsycTransfer,
};

const USYC: [u8; 20] = hex_literal::hex!("136471a34f6ef19fe571effc1ca711fdb8e49f2b");

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

            if log.address() == USYC.as_slice() {
                if let Some(ev) =
                    abi::usyc::events::Transfer::match_and_decode(log)
                {
                    events.usyc_transfers.push(UsycTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::usyc::events::FeeProcessed::match_and_decode(log)
                {
                    events.usyc_fee_processeds.push(UsycFeeProcessed {
                        id: id.clone(),
                        recipient: fmt_addr(&ev.recipient),
                        fee: ev.fee.to_string(),
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

    for e in events.usyc_transfers {
        tables
            .create_row("usyc_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.usyc_fee_processeds {
        tables
            .create_row("usyc_fee_processed", &e.id)
            .set("recipient", &e.recipient)
            .set("fee", &e.fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
