mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::mantle_staked_eth::types::v1::{
    Events, ReturnsAggregatorFeesCollected, StakingStaked, StakingUnstakeRequestClaimed, StakingUnstakeRequested,
};

const STAKING: [u8; 20] = hex_literal::hex!("e3cbd06d7dadb3f4e6557bab7edd924cd1489e8f");
const RETURNS_AGGREGATOR: [u8; 20] = hex_literal::hex!("1766be66fbb0a1883d41b4cfb0a533c5249d3b82");

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

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == STAKING.as_slice() {
                if let Some(ev) =
                    abi::staking::events::Staked::match_and_decode(log)
                {
                    events.staking_stakeds.push(StakingStaked {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        eth_amount: ev.eth_amount.to_string(),
                        m_eth_amount: ev.m_eth_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::staking::events::UnstakeRequested::match_and_decode(log)
                {
                    events.staking_unstake_requesteds.push(StakingUnstakeRequested {
                        id: id.clone(),
                        evt_id: ev.id.to_string(),
                        staker: fmt_addr(&ev.staker),
                        eth_amount: ev.eth_amount.to_string(),
                        m_eth_locked: ev.m_eth_locked.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::staking::events::UnstakeRequestClaimed::match_and_decode(log)
                {
                    events.staking_unstake_request_claimeds.push(StakingUnstakeRequestClaimed {
                        id: id.clone(),
                        evt_id: ev.id.to_string(),
                        staker: fmt_addr(&ev.staker),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == RETURNS_AGGREGATOR.as_slice() {
                if let Some(ev) =
                    abi::returns_aggregator::events::FeesCollected::match_and_decode(log)
                {
                    events.returns_aggregator_fees_collecteds.push(ReturnsAggregatorFeesCollected {
                        id: id.clone(),
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

    for e in events.staking_stakeds {
        tables
            .create_row("staking_staked", &e.id)
            .set("staker", &e.staker)
            .set("eth_amount", &e.eth_amount)
            .set("m_eth_amount", &e.m_eth_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.staking_unstake_requesteds {
        tables
            .create_row("staking_unstake_requested", &e.id)
            .set("evt_id", &e.evt_id)
            .set("staker", &e.staker)
            .set("eth_amount", &e.eth_amount)
            .set("m_eth_locked", &e.m_eth_locked)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.staking_unstake_request_claimeds {
        tables
            .create_row("staking_unstake_request_claimed", &e.id)
            .set("evt_id", &e.evt_id)
            .set("staker", &e.staker)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.returns_aggregator_fees_collecteds {
        tables
            .create_row("returns_aggregator_fees_collected", &e.id)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
