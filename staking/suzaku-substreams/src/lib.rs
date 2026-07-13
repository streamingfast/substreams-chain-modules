mod abi;
mod pb;

use substreams::errors::Error;
use substreams::store::{StoreGet, StoreGetString, StoreNew, StoreSet, StoreSetString};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::suzaku::types::v1::{Deposit, EntityAdded, Events, Withdraw};

const FACTORY: [u8; 20] = hex_literal::hex!("e5296638aa86bd4175d802a210e158688e41a93c");

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

// Record every Collateral contract spawned by the factory, keyed by its address.
#[substreams::handlers::store]
pub fn store_pools(block: Block, store: StoreSetString) {
    for trx in block.transactions() {
        for log in trx.receipt().logs() {
            if log.address() == FACTORY.as_slice() {
                if let Some(ev) =
                    abi::collateral_factory::events::AddEntity::match_and_decode(log)
                {
                    store.set(log.ordinal(), fmt_addr(&ev.entity), &"1".to_string());
                }
            }
        }
    }
}

#[substreams::handlers::map]
pub fn map_events(block: Block, store: StoreGetString) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == FACTORY.as_slice() {
                if let Some(ev) =
                    abi::collateral_factory::events::AddEntity::match_and_decode(log)
                {
                    events.entities_added.push(EntityAdded {
                        id: id.clone(),
                        entity: fmt_addr(&ev.entity),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            // Template events: only decode logs from factory-created Collateral contracts.
            if store.get_last(fmt_addr(log.address())).is_some() {
                let pool = fmt_addr(log.address());

                if let Some(ev) = abi::collateral::events::Deposit::match_and_decode(log) {
                    events.deposits.push(Deposit {
                        id: id.clone(),
                        pool: pool.clone(),
                        depositor: fmt_addr(&ev.depositor),
                        recipient: fmt_addr(&ev.recipient),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) = abi::collateral::events::Withdraw::match_and_decode(log) {
                    events.withdrawals.push(Withdraw {
                        id: id.clone(),
                        pool: pool.clone(),
                        withdrawer: fmt_addr(&ev.withdrawer),
                        recipient: fmt_addr(&ev.recipient),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.entities_added {
        tables
            .create_row("entities_added", &e.id)
            .set("entity", &e.entity)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.deposits {
        tables
            .create_row("deposits", &e.id)
            .set("pool", &e.pool)
            .set("depositor", &e.depositor)
            .set("recipient", &e.recipient)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawals {
        tables
            .create_row("withdrawals", &e.id)
            .set("pool", &e.pool)
            .set("withdrawer", &e.withdrawer)
            .set("recipient", &e.recipient)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
