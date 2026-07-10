mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::ora_protocol::types::v1::{
    Events, OraStakeRouterClaimWithdraw, OraStakeRouterRequestWithdraw, OraStakeRouterStake,
};

const ORA_STAKE_ROUTER: [u8; 20] = hex_literal::hex!("784fdebfd4779579b4cc2bac484129d29200412a");

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

            if log.address == ORA_STAKE_ROUTER {
                if let Some(ev) =
                    abi::ora_stake_router::events::Stake::match_and_decode(log)
                {
                    events.ora_stake_router_stakes.push(OraStakeRouterStake {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        amount: ev.amount.to_string(),
                        pool: fmt_addr(&ev.pool),
                        vault_id: ev.vault_id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ora_stake_router::events::RequestWithdraw::match_and_decode(log)
                {
                    events.ora_stake_router_request_withdraws.push(OraStakeRouterRequestWithdraw {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        amount: ev.amount.to_string(),
                        pool: fmt_addr(&ev.pool),
                        vault_id: ev.vault_id.to_string(),
                        request_id: ev.request_id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ora_stake_router::events::ClaimWithdraw::match_and_decode(log)
                {
                    events.ora_stake_router_claim_withdraws.push(OraStakeRouterClaimWithdraw {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        amount: ev.amount.to_string(),
                        pool: fmt_addr(&ev.pool),
                        vault_id: ev.vault_id.to_string(),
                        last_request_id: ev.last_request_id.to_string(),
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

    for e in events.ora_stake_router_stakes {
        tables
            .create_row("ora_stake_router_stake", &e.id)
            .set("user", &e.user)
            .set("amount", &e.amount)
            .set("pool", &e.pool)
            .set("vault_id", &e.vault_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ora_stake_router_request_withdraws {
        tables
            .create_row("ora_stake_router_request_withdraw", &e.id)
            .set("user", &e.user)
            .set("amount", &e.amount)
            .set("pool", &e.pool)
            .set("vault_id", &e.vault_id)
            .set("request_id", &e.request_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ora_stake_router_claim_withdraws {
        tables
            .create_row("ora_stake_router_claim_withdraw", &e.id)
            .set("user", &e.user)
            .set("amount", &e.amount)
            .set("pool", &e.pool)
            .set("vault_id", &e.vault_id)
            .set("last_request_id", &e.last_request_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
