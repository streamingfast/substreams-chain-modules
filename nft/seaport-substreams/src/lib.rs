mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::seaport::types::v1::{Events, OrderFulfilled, ReceivedItem, SpentItem};

const SEAPORT: [u8; 20] = hex_literal::hex!("00000000006c3852cbef3e08e8df289169ede581");

fn fmt_addr(addr: &[u8]) -> String {
    format!("0x{}", hex::encode(addr))
}

fn fmt_bytes32(b: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(b))
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
            if log.address != SEAPORT {
                continue;
            }

            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) =
                abi::seaport_exchange::events::OrderFulfilled::match_and_decode(log)
            {
                let offer = ev
                    .offer
                    .iter()
                    .map(|(item_type, token, identifier, amount)| SpentItem {
                        item_type: item_type.to_u64() as u32,
                        token: fmt_addr(token),
                        identifier: identifier.to_string(),
                        amount: amount.to_string(),
                    })
                    .collect();

                let consideration = ev
                    .consideration
                    .iter()
                    .map(|(item_type, token, identifier, amount, recipient)| ReceivedItem {
                        item_type: item_type.to_u64() as u32,
                        token: fmt_addr(token),
                        identifier: identifier.to_string(),
                        amount: amount.to_string(),
                        recipient: fmt_addr(recipient),
                    })
                    .collect();

                events.orders_fulfilled.push(OrderFulfilled {
                    id,
                    order_hash: fmt_bytes32(&ev.order_hash),
                    offerer: fmt_addr(&ev.offerer),
                    zone: fmt_addr(&ev.zone),
                    recipient: fmt_addr(&ev.recipient),
                    offer,
                    consideration,
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

    for e in events.orders_fulfilled {
        let row = tables
            .create_row("orders_fulfilled", &e.id)
            .set("order_hash", &e.order_hash)
            .set("offerer", &e.offerer)
            .set("zone", &e.zone)
            .set("recipient", &e.recipient)
            .set("offer_count", e.offer.len() as i64)
            .set("consideration_count", e.consideration.len() as i64)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
        let _ = row;
    }

    Ok(tables.to_database_changes())
}
