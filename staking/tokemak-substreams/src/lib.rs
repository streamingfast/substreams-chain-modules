mod abi;
mod pb;

use substreams::errors::Error;
use substreams::store::{StoreGet, StoreGetString, StoreNew, StoreSet, StoreSetString};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::tokemak::types::v1::{
    Events, ManagerPoolRegistered, RewardsClaimed, VaultWithdrawalRequested,
};

const FACTORY: [u8; 20] = hex_literal::hex!("a86e412109f77c45a3bc1c5870b880492fb86a14");
const REWARDS: [u8; 20] = hex_literal::hex!("79dd22579112d8a5f7347c5ed7e609e60da713c5");
const ON_CHAIN_VOTE: [u8; 20] = hex_literal::hex!("43094ed6d6d214e43c31c38da91231d2296ca511");

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

#[substreams::handlers::store]
pub fn store_pools(block: Block, store: StoreSetString) {
    for trx in block.transactions() {
        for log in trx.receipt().logs() {
            if log.address() == FACTORY.as_slice() {
                if let Some(ev) =
                    abi::manager::events::PoolRegistered::match_and_decode(log)
                {
                    store.set(log.ordinal(), fmt_addr(&ev.pool), &"1".to_string());
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
                    abi::manager::events::PoolRegistered::match_and_decode(log)
                {
                    events.manager_pool_registereds.push(ManagerPoolRegistered {
                        id: id.clone(),
                        pool: fmt_addr(&ev.pool),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if store.get_last(fmt_addr(log.address())).is_some() {
                let pool = fmt_addr(log.address());
                if let Some(ev) =
                    abi::vault::events::WithdrawalRequested::match_and_decode(log)
                {
                    events.vault_withdrawal_requesteds.push(VaultWithdrawalRequested {
                        id: id.clone(),
                        pool: pool.clone(),
                        requestor: fmt_addr(&ev.requestor),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == REWARDS.as_slice() {
                if let Some(ev) =
                    abi::rewards::events::Claimed::match_and_decode(log)
                {
                    events.rewards_claimeds.push(RewardsClaimed {
                        id: id.clone(),
                        cycle: ev.cycle.to_string(),
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

            if log.address() == ON_CHAIN_VOTE.as_slice() {
            }

        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.manager_pool_registereds {
        tables
            .create_row("manager_pool_registered", &e.id)
            .set("pool", &e.pool)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.vault_withdrawal_requesteds {
        tables
            .create_row("vault_withdrawal_requested", &e.id)
            .set("pool", &e.pool)
            .set("requestor", &e.requestor)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rewards_claimeds {
        tables
            .create_row("rewards_claimed", &e.id)
            .set("cycle", &e.cycle)
            .set("recipient", &e.recipient)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
