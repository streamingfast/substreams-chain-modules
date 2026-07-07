mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::cbeth::types::v1::{Burn, Events, ExchangeRateUpdated, Mint};

const CBETH: [u8; 20] = hex_literal::hex!("be9895146f7af43049ca1c1ae358b0541ea49704");

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
            if log.address != CBETH {
                continue;
            }

            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) = abi::cbeth::events::Mint::match_and_decode(log) {
                events.mints.push(Mint {
                    id,
                    minter: fmt_addr(&ev.minter),
                    to: fmt_addr(&ev.to),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::cbeth::events::Burn::match_and_decode(log) {
                events.burns.push(Burn {
                    id,
                    burner: fmt_addr(&ev.burner),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::cbeth::events::ExchangeRateUpdated::match_and_decode(log) {
                events.exchange_rate_updates.push(ExchangeRateUpdated {
                    id,
                    oracle: fmt_addr(&ev.oracle),
                    new_exchange_rate: ev.new_exchange_rate.to_string(),
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

    for e in events.mints {
        tables
            .create_row("mints", &e.id)
            .set("minter", &e.minter)
            .set("to", &e.to)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.burns {
        tables
            .create_row("burns", &e.id)
            .set("burner", &e.burner)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.exchange_rate_updates {
        tables
            .create_row("exchange_rate_updates", &e.id)
            .set("oracle", &e.oracle)
            .set("new_exchange_rate", &e.new_exchange_rate)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
