mod abi;
mod pb;

use substreams::errors::Error;
use substreams::store::{StoreGet, StoreGetString, StoreNew, StoreSetIfNotExists, StoreSetIfNotExistsString};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::velodrome::types::v1::{Events, LiquidityEvent, Pool, Pools, Swap};

// Velodrome V2 PoolFactory on Optimism — deployed block 119142390
const POOL_FACTORY: [u8; 20] = hex_literal::hex!("F1046053aa5682b4F9a81b5481394DA16BE5FF5a");

fn block_timestamp(block: &Block) -> u64 {
    block
        .header
        .as_ref()
        .and_then(|h| h.timestamp.as_ref().map(|t| t.seconds as u64))
        .unwrap_or(0)
}

#[substreams::handlers::map]
pub fn map_factory_events(block: Block) -> Result<Pools, Error> {
    let mut pools = Pools::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));
        for (log, _call) in trx.logs_with_calls() {
            if log.address != POOL_FACTORY {
                continue;
            }
            if let Some(ev) = abi::pool_factory::events::PoolCreated::match_and_decode(log) {
                pools.pools.push(Pool {
                    address: format!("0x{}", hex::encode(ev.pool)),
                    token0: format!("0x{}", hex::encode(ev.token0)),
                    token1: format!("0x{}", hex::encode(ev.token1)),
                    stable: ev.stable,
                    tx_hash: tx_hash.clone(),
                    block_num: block.number,
                    timestamp,
                });
            }
        }
    }

    Ok(pools)
}

#[substreams::handlers::store]
pub fn store_pools(pools: Pools, store: StoreSetIfNotExistsString) {
    for pool in pools.pools {
        store.set_if_not_exists(0, &pool.address, &"1".to_string());
    }
}

#[substreams::handlers::map]
pub fn map_pool_events(block: Block, store: StoreGetString) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));
        for (log, _call) in trx.logs_with_calls() {
            let pool_addr = format!("0x{}", hex::encode(&log.address));

            if store.get_last(&pool_addr).is_none() {
                continue;
            }

            if let Some(ev) = abi::pool::events::Swap::match_and_decode(log) {
                events.swaps.push(Swap {
                    pool: pool_addr.clone(),
                    sender: format!("0x{}", hex::encode(ev.sender)),
                    to: format!("0x{}", hex::encode(ev.to)),
                    amount0_in: ev.amount0_in.to_string(),
                    amount1_in: ev.amount1_in.to_string(),
                    amount0_out: ev.amount0_out.to_string(),
                    amount1_out: ev.amount1_out.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::pool::events::Mint::match_and_decode(log) {
                events.liquidity.push(LiquidityEvent {
                    event_type: "mint".to_string(),
                    pool: pool_addr.clone(),
                    sender: format!("0x{}", hex::encode(ev.sender)),
                    amount0: ev.amount0.to_string(),
                    amount1: ev.amount1.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::pool::events::Burn::match_and_decode(log) {
                events.liquidity.push(LiquidityEvent {
                    event_type: "burn".to_string(),
                    pool: pool_addr.clone(),
                    sender: format!("0x{}", hex::encode(ev.sender)),
                    amount0: ev.amount0.to_string(),
                    amount1: ev.amount1.to_string(),
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
pub fn db_out(pools: Pools, events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for pool in pools.pools {
        tables
            .create_row("pools", &pool.address)
            .set("token0", &pool.token0)
            .set("token1", &pool.token1)
            .set("stable", pool.stable)
            .set("block_num", pool.block_num as i64)
            .set("tx_hash", &pool.tx_hash);
    }

    for swap in events.swaps {
        let id = format!("{}-{}", swap.tx_hash, swap.log_index);
        tables
            .create_row("swaps", &id)
            .set("pool", &swap.pool)
            .set("sender", &swap.sender)
            .set("to", &swap.to)
            .set("amount0_in", &swap.amount0_in)
            .set("amount1_in", &swap.amount1_in)
            .set("amount0_out", &swap.amount0_out)
            .set("amount1_out", &swap.amount1_out)
            .set("block_num", swap.block_num as i64)
            .set("timestamp", swap.timestamp as i64)
            .set("tx_hash", &swap.tx_hash)
            .set("log_index", swap.log_index as i64);
    }

    for liq in events.liquidity {
        let id = format!("{}-{}", liq.tx_hash, liq.log_index);
        tables
            .create_row("liquidity_events", &id)
            .set("event_type", &liq.event_type)
            .set("pool", &liq.pool)
            .set("sender", &liq.sender)
            .set("amount0", &liq.amount0)
            .set("amount1", &liq.amount1)
            .set("block_num", liq.block_num as i64)
            .set("timestamp", liq.timestamp as i64)
            .set("tx_hash", &liq.tx_hash)
            .set("log_index", liq.log_index as i64);
    }

    Ok(tables.to_database_changes())
}
