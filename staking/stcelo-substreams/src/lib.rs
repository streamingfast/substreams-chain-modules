mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::stcelo::types::v1::{
    Events, StceloCeloWithdrawalScheduled, StceloCeloWithdrawalStarted, StceloVotesScheduled,
};

const STCELO: [u8; 20] = hex_literal::hex!("4aad04d41fd7fd495503731c5a2579e19054c432");

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

            if log.address == STCELO {
                if let Some(ev) =
                    abi::stcelo::events::VotesScheduled::match_and_decode(log)
                {
                    events.stcelo_votes_scheduleds.push(StceloVotesScheduled {
                        id: id.clone(),
                        group: fmt_addr(&ev.group),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::stcelo::events::CeloWithdrawalScheduled::match_and_decode(log)
                {
                    events.stcelo_celo_withdrawal_scheduleds.push(StceloCeloWithdrawalScheduled {
                        id: id.clone(),
                        beneficiary: fmt_addr(&ev.beneficiary),
                        group: fmt_addr(&ev.group),
                        withdrawal_amount: ev.withdrawal_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::stcelo::events::CeloWithdrawalStarted::match_and_decode(log)
                {
                    events.stcelo_celo_withdrawal_starteds.push(StceloCeloWithdrawalStarted {
                        id: id.clone(),
                        beneficiary: fmt_addr(&ev.beneficiary),
                        group: fmt_addr(&ev.group),
                        withdrawal_amount: ev.withdrawal_amount.to_string(),
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

    for e in events.stcelo_votes_scheduleds {
        tables
            .create_row("stcelo_votes_scheduled", &e.id)
            .set("group", &e.group)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stcelo_celo_withdrawal_scheduleds {
        tables
            .create_row("stcelo_celo_withdrawal_scheduled", &e.id)
            .set("beneficiary", &e.beneficiary)
            .set("group", &e.group)
            .set("withdrawal_amount", &e.withdrawal_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stcelo_celo_withdrawal_starteds {
        tables
            .create_row("stcelo_celo_withdrawal_started", &e.id)
            .set("beneficiary", &e.beneficiary)
            .set("group", &e.group)
            .set("withdrawal_amount", &e.withdrawal_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
