mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::cryptopunks::types::v1::{Bid, Events, Trade};

// CryptoPunks contract on Ethereum mainnet — deployed block 3914495
const CRYPTOPUNKS: [u8; 20] = hex_literal::hex!("b47e3cd837dDF8e4c57F05d70Ab865de6e193BBB");

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
            if log.address != CRYPTOPUNKS {
                continue;
            }

            if let Some(ev) = abi::cryptopunks::events::PunkBought::match_and_decode(log) {
                let id = format!("{}-{}", tx_hash, log.index);
                events.trades.push(Trade {
                    id,
                    punk_index: ev.punk_index.to_u64(),
                    price_eth_raw: ev.value.to_string(),
                    buyer: format!("0x{}", hex::encode(ev.to_address)),
                    seller: format!("0x{}", hex::encode(ev.from_address)),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::cryptopunks::events::PunkBidEntered::match_and_decode(log) {
                let id = format!("{}-{}", tx_hash, log.index);
                events.bids.push(Bid {
                    id,
                    punk_index: ev.punk_index.to_u64(),
                    amount_raw: ev.value.to_string(),
                    bidder: format!("0x{}", hex::encode(ev.from_address)),
                    event_type: "entered".to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::cryptopunks::events::PunkBidWithdrawn::match_and_decode(log) {
                let id = format!("{}-{}", tx_hash, log.index);
                events.bids.push(Bid {
                    id,
                    punk_index: ev.punk_index.to_u64(),
                    amount_raw: ev.value.to_string(),
                    bidder: format!("0x{}", hex::encode(ev.from_address)),
                    event_type: "withdrawn".to_string(),
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

    for trade in events.trades {
        tables
            .create_row("trades", &trade.id)
            .set("punk_index", trade.punk_index as i64)
            .set("price_eth_raw", &trade.price_eth_raw)
            .set("buyer", &trade.buyer)
            .set("seller", &trade.seller)
            .set("tx_hash", &trade.tx_hash)
            .set("log_index", trade.log_index as i64)
            .set("block_num", trade.block_num as i64)
            .set("timestamp", trade.timestamp as i64);
    }

    for bid in events.bids {
        tables
            .create_row("bids", &bid.id)
            .set("punk_index", bid.punk_index as i64)
            .set("amount_raw", &bid.amount_raw)
            .set("bidder", &bid.bidder)
            .set("event_type", &bid.event_type)
            .set("tx_hash", &bid.tx_hash)
            .set("log_index", bid.log_index as i64)
            .set("block_num", bid.block_num as i64)
            .set("timestamp", bid.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
