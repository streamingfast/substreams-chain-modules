mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::looksrare::types::v1::{
    CancelAllOrders, CancelMultipleOrders, Events, RoyaltyPayment, TakerAsk, TakerBid,
};

// LooksRare Exchange v1 on Ethereum mainnet
const LOOKSRARE_EXCHANGE: [u8; 20] =
    hex_literal::hex!("59728544b08ab483533076417fbbb2fd0b17ce3a");

fn block_timestamp(block: &Block) -> u64 {
    block
        .header
        .as_ref()
        .and_then(|h| h.timestamp.as_ref().map(|t| t.seconds as u64))
        .unwrap_or(0)
}

fn fmt_addr(addr: &[u8]) -> String {
    format!("0x{}", hex::encode(addr))
}

fn fmt_bytes32(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for (log, _call) in trx.logs_with_calls() {
            if log.address != LOOKSRARE_EXCHANGE {
                continue;
            }

            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) =
                abi::looksrare_exchange::events::TakerAsk::match_and_decode(log)
            {
                events.taker_asks.push(TakerAsk {
                    id,
                    order_hash: fmt_bytes32(&ev.order_hash),
                    order_nonce: ev.order_nonce.to_string(),
                    taker: fmt_addr(&ev.taker),
                    maker: fmt_addr(&ev.maker),
                    strategy: fmt_addr(&ev.strategy),
                    currency: fmt_addr(&ev.currency),
                    collection: fmt_addr(&ev.collection),
                    token_id: ev.token_id.to_string(),
                    amount: ev.amount.to_string(),
                    price: ev.price.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::looksrare_exchange::events::TakerBid::match_and_decode(log)
            {
                events.taker_bids.push(TakerBid {
                    id,
                    order_hash: fmt_bytes32(&ev.order_hash),
                    order_nonce: ev.order_nonce.to_string(),
                    taker: fmt_addr(&ev.taker),
                    maker: fmt_addr(&ev.maker),
                    strategy: fmt_addr(&ev.strategy),
                    currency: fmt_addr(&ev.currency),
                    collection: fmt_addr(&ev.collection),
                    token_id: ev.token_id.to_string(),
                    amount: ev.amount.to_string(),
                    price: ev.price.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::looksrare_exchange::events::RoyaltyPayment::match_and_decode(log)
            {
                events.royalty_payments.push(RoyaltyPayment {
                    id,
                    collection: fmt_addr(&ev.collection),
                    token_id: ev.token_id.to_string(),
                    royalty_recipient: fmt_addr(&ev.royalty_recipient),
                    currency: fmt_addr(&ev.currency),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::looksrare_exchange::events::CancelAllOrders::match_and_decode(log)
            {
                events.cancel_all_orders.push(CancelAllOrders {
                    id,
                    user: fmt_addr(&ev.user),
                    new_min_nonce: ev.new_min_nonce.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::looksrare_exchange::events::CancelMultipleOrders::match_and_decode(log)
            {
                events.cancel_multiple_orders.push(CancelMultipleOrders {
                    id,
                    user: fmt_addr(&ev.user),
                    order_nonces: ev.order_nonces.iter().map(|n| n.to_string()).collect(),
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

    for e in events.taker_asks {
        tables
            .create_row("taker_asks", &e.id)
            .set("order_hash", &e.order_hash)
            .set("order_nonce", &e.order_nonce)
            .set("taker", &e.taker)
            .set("maker", &e.maker)
            .set("strategy", &e.strategy)
            .set("currency", &e.currency)
            .set("collection", &e.collection)
            .set("token_id", &e.token_id)
            .set("amount", &e.amount)
            .set("price", &e.price)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.taker_bids {
        tables
            .create_row("taker_bids", &e.id)
            .set("order_hash", &e.order_hash)
            .set("order_nonce", &e.order_nonce)
            .set("taker", &e.taker)
            .set("maker", &e.maker)
            .set("strategy", &e.strategy)
            .set("currency", &e.currency)
            .set("collection", &e.collection)
            .set("token_id", &e.token_id)
            .set("amount", &e.amount)
            .set("price", &e.price)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.royalty_payments {
        tables
            .create_row("royalty_payments", &e.id)
            .set("collection", &e.collection)
            .set("token_id", &e.token_id)
            .set("royalty_recipient", &e.royalty_recipient)
            .set("currency", &e.currency)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.cancel_all_orders {
        tables
            .create_row("cancel_all_orders", &e.id)
            .set("user", &e.user)
            .set("new_min_nonce", &e.new_min_nonce)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.cancel_multiple_orders {
        tables
            .create_row("cancel_multiple_orders", &e.id)
            .set("user", &e.user)
            .set("order_nonces", &e.order_nonces.join(","))
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
