mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::tensorplex::types::v1::{
    Events, PlxtaoUserStake, PlxtaoUserUnstake, PlxtaoUserUnstakeRequested,
};

const PLXTAO: [u8; 20] = hex_literal::hex!("b60acd2057067dc9ed8c083f5aa227a244044fd6");

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

            if log.address == PLXTAO {
                if let Some(ev) =
                    abi::plxtao::events::UserStake::match_and_decode(log)
                {
                    events.plxtao_user_stakes.push(PlxtaoUserStake {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        stake_timestamp: ev.stake_timestamp.to_string(),
                        in_tao_amt: ev.in_tao_amt.to_string(),
                        wst_amount: ev.wst_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::plxtao::events::UserUnstake::match_and_decode(log)
                {
                    events.plxtao_user_unstakes.push(PlxtaoUserUnstake {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        idx: ev.idx.to_string(),
                        unstake_timestamp: ev.unstake_timestamp.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::plxtao::events::UserUnstakeRequested::match_and_decode(log)
                {
                    events.plxtao_user_unstake_requesteds.push(PlxtaoUserUnstakeRequested {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        idx: ev.idx.to_string(),
                        request_timestamp: ev.request_timestamp.to_string(),
                        wst_amount: ev.wst_amount.to_string(),
                        out_tao_amt: ev.out_tao_amt.to_string(),
                        wrapped_token: fmt_addr(&ev.wrapped_token),
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

    for e in events.plxtao_user_stakes {
        tables
            .create_row("plxtao_user_stake", &e.id)
            .set("user", &e.user)
            .set("stake_timestamp", &e.stake_timestamp)
            .set("in_tao_amt", &e.in_tao_amt)
            .set("wst_amount", &e.wst_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.plxtao_user_unstakes {
        tables
            .create_row("plxtao_user_unstake", &e.id)
            .set("user", &e.user)
            .set("idx", &e.idx)
            .set("unstake_timestamp", &e.unstake_timestamp)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.plxtao_user_unstake_requesteds {
        tables
            .create_row("plxtao_user_unstake_requested", &e.id)
            .set("user", &e.user)
            .set("idx", &e.idx)
            .set("request_timestamp", &e.request_timestamp)
            .set("wst_amount", &e.wst_amount)
            .set("out_tao_amt", &e.out_tao_amt)
            .set("wrapped_token", &e.wrapped_token)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
