mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::lybra_finance::types::v1::{
    Events, LybraV1Burn, LybraV1FeeDistribution, LybraV1Mint,
};

const LYBRA_V1: [u8; 20] = hex_literal::hex!("97de57ec338ab5d51557da3434828c5dbfada371");

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

            if log.address == LYBRA_V1 {
                if let Some(ev) =
                    abi::lybra_v1::events::Mint::match_and_decode(log)
                {
                    events.lybra_v1_mints.push(LybraV1Mint {
                        id: id.clone(),
                        sponsor: fmt_addr(&ev.sponsor),
                        on_behalf_of: fmt_addr(&ev.on_behalf_of),
                        amount: ev.amount.to_string(),
                        evt_timestamp: ev.timestamp.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lybra_v1::events::Burn::match_and_decode(log)
                {
                    events.lybra_v1_burns.push(LybraV1Burn {
                        id: id.clone(),
                        sponsor: fmt_addr(&ev.sponsor),
                        on_behalf_of: fmt_addr(&ev.on_behalf_of),
                        amount: ev.amount.to_string(),
                        evt_timestamp: ev.timestamp.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lybra_v1::events::FeeDistribution::match_and_decode(log)
                {
                    events.lybra_v1_fee_distributions.push(LybraV1FeeDistribution {
                        id: id.clone(),
                        fee_address: fmt_addr(&ev.fee_address),
                        fee_amount: ev.fee_amount.to_string(),
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

    for e in events.lybra_v1_mints {
        tables
            .create_row("lybra_v1_mint", &e.id)
            .set("sponsor", &e.sponsor)
            .set("on_behalf_of", &e.on_behalf_of)
            .set("amount", &e.amount)
            .set("evt_timestamp", &e.evt_timestamp)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lybra_v1_burns {
        tables
            .create_row("lybra_v1_burn", &e.id)
            .set("sponsor", &e.sponsor)
            .set("on_behalf_of", &e.on_behalf_of)
            .set("amount", &e.amount)
            .set("evt_timestamp", &e.evt_timestamp)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lybra_v1_fee_distributions {
        tables
            .create_row("lybra_v1_fee_distribution", &e.id)
            .set("fee_address", &e.fee_address)
            .set("fee_amount", &e.fee_amount)
            .set("evt_timestamp", &e.evt_timestamp)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
