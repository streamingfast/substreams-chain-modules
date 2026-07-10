mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::powh3d::types::v1::{
    Events, HourglassOnReinvestment, HourglassOnTokenPurchase, HourglassOnTokenSell, HourglassOnWithdraw, HourglassTransfer,
};

const HOURGLASS: [u8; 20] = hex_literal::hex!("b3775fb83f7d12a36e0475abdd1fca35c091efbe");

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

            if log.address == HOURGLASS {
                if let Some(ev) =
                    abi::hourglass::events::OnTokenPurchase::match_and_decode(log)
                {
                    events.hourglass_on_token_purchases.push(HourglassOnTokenPurchase {
                        id: id.clone(),
                        customer_address: fmt_addr(&ev.customer_address),
                        incoming_ethereum: ev.incoming_ethereum.to_string(),
                        tokens_minted: ev.tokens_minted.to_string(),
                        referred_by: fmt_addr(&ev.referred_by),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::hourglass::events::OnTokenSell::match_and_decode(log)
                {
                    events.hourglass_on_token_sells.push(HourglassOnTokenSell {
                        id: id.clone(),
                        customer_address: fmt_addr(&ev.customer_address),
                        tokens_burned: ev.tokens_burned.to_string(),
                        ethereum_earned: ev.ethereum_earned.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::hourglass::events::OnReinvestment::match_and_decode(log)
                {
                    events.hourglass_on_reinvestments.push(HourglassOnReinvestment {
                        id: id.clone(),
                        customer_address: fmt_addr(&ev.customer_address),
                        ethereum_reinvested: ev.ethereum_reinvested.to_string(),
                        tokens_minted: ev.tokens_minted.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::hourglass::events::OnWithdraw::match_and_decode(log)
                {
                    events.hourglass_on_withdraws.push(HourglassOnWithdraw {
                        id: id.clone(),
                        customer_address: fmt_addr(&ev.customer_address),
                        ethereum_withdrawn: ev.ethereum_withdrawn.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::hourglass::events::Transfer::match_and_decode(log)
                {
                    events.hourglass_transfers.push(HourglassTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        tokens: ev.tokens.to_string(),
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

    for e in events.hourglass_on_token_purchases {
        tables
            .create_row("hourglass_on_token_purchase", &e.id)
            .set("customer_address", &e.customer_address)
            .set("incoming_ethereum", &e.incoming_ethereum)
            .set("tokens_minted", &e.tokens_minted)
            .set("referred_by", &e.referred_by)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.hourglass_on_token_sells {
        tables
            .create_row("hourglass_on_token_sell", &e.id)
            .set("customer_address", &e.customer_address)
            .set("tokens_burned", &e.tokens_burned)
            .set("ethereum_earned", &e.ethereum_earned)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.hourglass_on_reinvestments {
        tables
            .create_row("hourglass_on_reinvestment", &e.id)
            .set("customer_address", &e.customer_address)
            .set("ethereum_reinvested", &e.ethereum_reinvested)
            .set("tokens_minted", &e.tokens_minted)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.hourglass_on_withdraws {
        tables
            .create_row("hourglass_on_withdraw", &e.id)
            .set("customer_address", &e.customer_address)
            .set("ethereum_withdrawn", &e.ethereum_withdrawn)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.hourglass_transfers {
        tables
            .create_row("hourglass_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("tokens", &e.tokens)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
