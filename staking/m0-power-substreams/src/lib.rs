mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::m0_power::types::v1::{
    Events, MinterCollateralUpdated,
};

const MINTER: [u8; 20] = hex_literal::hex!("f7f9638cb444d65e5a40bf5ff98ebe4ff319f04e");

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

            if log.address == MINTER {
                if let Some(ev) =
                    abi::minter::events::CollateralUpdated::match_and_decode(log)
                {
                    events.minter_collateral_updateds.push(MinterCollateralUpdated {
                        id: id.clone(),
                        minter: fmt_addr(&ev.minter),
                        collateral: ev.collateral.to_string(),
                        total_resolved_collateral_retrieval: ev.total_resolved_collateral_retrieval.to_string(),
                        metadata_hash: fmt_addr(&ev.metadata_hash),
                        evt_timestamp: ev.timestamp.to_string(),
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

    for e in events.minter_collateral_updateds {
        tables
            .create_row("minter_collateral_updated", &e.id)
            .set("minter", &e.minter)
            .set("collateral", &e.collateral)
            .set("total_resolved_collateral_retrieval", &e.total_resolved_collateral_retrieval)
            .set("metadata_hash", &e.metadata_hash)
            .set("evt_timestamp", &e.evt_timestamp)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
