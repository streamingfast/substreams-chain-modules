mod abi;
// buffa emits view re-exports for every message; most modules use only the owned type.
#[allow(unused_imports)]
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::aqua_patina::types::v1::{ApethEarlyDepositsMinted, ApethTransfer, Events};

const APETH: [u8; 20] = hex_literal::hex!("aaaaaaabc6cbc3a1fd3a0fe0fdec43251c6562f5");
const APETH_EARLY_DEPOSITS: [u8; 20] = hex_literal::hex!("9b4f8873090fcb3b9d5e562afcdcbdf30228f301");

fn fmt_addr(addr: &[u8]) -> String {
    format!("0x{}", hex::encode(addr))
}

fn block_timestamp(block: &Block) -> u64 {
    block.header.timestamp.seconds as u64
}

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == APETH.as_slice() {
                if let Some(ev) = abi::apeth::events::Transfer::match_and_decode(log) {
                    events.apeth_transfers.push(ApethTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        value: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == APETH_EARLY_DEPOSITS.as_slice() {
                if let Some(ev) = abi::apeth_early_deposits::events::Minted::match_and_decode(log) {
                    events.apeth_early_deposits_minteds.push(ApethEarlyDepositsMinted {
                        id: id.clone(),
                        recipient: fmt_addr(&ev.recipient),
                        amount: ev.amount.to_string(),
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

    for e in events.apeth_transfers {
        tables
            .create_row("apeth_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.apeth_early_deposits_minteds {
        tables
            .create_row("apeth_early_deposits_minted", &e.id)
            .set("recipient", &e.recipient)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
