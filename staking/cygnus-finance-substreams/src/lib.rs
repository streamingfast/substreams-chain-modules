mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::cygnus_finance::types::v1::{
    Events, CgusdInvested, CgusdSharesBurnt, CgusdSubmitted,
};

const CGUSD: [u8; 20] = hex_literal::hex!("ca72827a3d211cfd8f6b00ac98824872b72cab49");

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

            if log.address == CGUSD {
                if let Some(ev) =
                    abi::cgusd::events::Invested::match_and_decode(log)
                {
                    events.cgusd_investeds.push(CgusdInvested {
                        id: id.clone(),
                        amount: ev.amount.to_string(),
                        post_buffered_assets: ev.post_buffered_assets.to_string(),
                        post_invested_assets: ev.post_invested_assets.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::cgusd::events::SharesBurnt::match_and_decode(log)
                {
                    events.cgusd_shares_burnts.push(CgusdSharesBurnt {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        pre_rebase_token_amount: ev.pre_rebase_token_amount.to_string(),
                        post_rebase_token_amount: ev.post_rebase_token_amount.to_string(),
                        shares_amount: ev.shares_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::cgusd::events::Submitted::match_and_decode(log)
                {
                    events.cgusd_submitteds.push(CgusdSubmitted {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
                        referral: fmt_addr(&ev.referral),
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

    for e in events.cgusd_investeds {
        tables
            .create_row("cgusd_invested", &e.id)
            .set("amount", &e.amount)
            .set("post_buffered_assets", &e.post_buffered_assets)
            .set("post_invested_assets", &e.post_invested_assets)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.cgusd_shares_burnts {
        tables
            .create_row("cgusd_shares_burnt", &e.id)
            .set("account", &e.account)
            .set("pre_rebase_token_amount", &e.pre_rebase_token_amount)
            .set("post_rebase_token_amount", &e.post_rebase_token_amount)
            .set("shares_amount", &e.shares_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.cgusd_submitteds {
        tables
            .create_row("cgusd_submitted", &e.id)
            .set("sender", &e.sender)
            .set("amount", &e.amount)
            .set("referral", &e.referral)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
