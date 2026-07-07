mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::rocketpool::types::v1::{EtherDeposited, Events, TokensBurned, TokensMinted};

const RETH: [u8; 20] = hex_literal::hex!("ae78736cd615f374d3085123a210448e74fc6393");

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
            if log.address != RETH {
                continue;
            }

            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) =
                abi::rocket_token_reth::events::EtherDeposited::match_and_decode(log)
            {
                events.ether_deposited.push(EtherDeposited {
                    id,
                    from: fmt_addr(&ev.from),
                    amount: ev.amount.to_string(),
                    time: ev.time.to_u64(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::rocket_token_reth::events::TokensMinted::match_and_decode(log)
            {
                events.tokens_minted.push(TokensMinted {
                    id,
                    to: fmt_addr(&ev.to),
                    amount: ev.amount.to_string(),
                    eth_amount: ev.eth_amount.to_string(),
                    time: ev.time.to_u64(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::rocket_token_reth::events::TokensBurned::match_and_decode(log)
            {
                events.tokens_burned.push(TokensBurned {
                    id,
                    from: fmt_addr(&ev.from),
                    amount: ev.amount.to_string(),
                    eth_amount: ev.eth_amount.to_string(),
                    time: ev.time.to_u64(),
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

    for e in events.ether_deposited {
        tables
            .create_row("ether_deposited", &e.id)
            .set("from", &e.from)
            .set("amount", &e.amount)
            .set("time", e.time as i64)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.tokens_minted {
        tables
            .create_row("tokens_minted", &e.id)
            .set("to", &e.to)
            .set("amount", &e.amount)
            .set("eth_amount", &e.eth_amount)
            .set("time", e.time as i64)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.tokens_burned {
        tables
            .create_row("tokens_burned", &e.id)
            .set("from", &e.from)
            .set("amount", &e.amount)
            .set("eth_amount", &e.eth_amount)
            .set("time", e.time as i64)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
