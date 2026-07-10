mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::hord::types::v1::{
    Events, HethStakingStatsUpdated, HethTransfer,
};

const HETH: [u8; 20] = hex_literal::hex!("5bbe36152d3cd3eb7183a82470b39b29eedf068b");

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

            if log.address == HETH {
                if let Some(ev) =
                    abi::heth::events::Transfer::match_and_decode(log)
                {
                    events.heth_transfers.push(HethTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        value: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::heth::events::StakingStatsUpdated::match_and_decode(log)
                {
                    events.heth_staking_stats_updateds.push(HethStakingStatsUpdated {
                        id: id.clone(),
                        new_rewards_amount: ev.new_rewards_amount.to_string(),
                        new_total_eth_balance_in_validators: ev.new_total_eth_balance_in_validators.to_string(),
                        new_total_execution_layer_rewards: ev.new_total_execution_layer_rewards.to_string(),
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

    for e in events.heth_transfers {
        tables
            .create_row("heth_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.heth_staking_stats_updateds {
        tables
            .create_row("heth_staking_stats_updated", &e.id)
            .set("new_rewards_amount", &e.new_rewards_amount)
            .set("new_total_eth_balance_in_validators", &e.new_total_eth_balance_in_validators)
            .set("new_total_execution_layer_rewards", &e.new_total_execution_layer_rewards)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
