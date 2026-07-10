mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::amphor::types::v1::{
    Events, LrtVaultDeposit, LrtVaultWithdraw,
};

const LRT_VAULT: [u8; 20] = hex_literal::hex!("06824c27c8a0dbde5f72f770ec82e3c0fd4dcec3");

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

            if log.address == LRT_VAULT {
                if let Some(ev) =
                    abi::lrt_vault::events::Deposit::match_and_decode(log)
                {
                    events.lrt_vault_deposits.push(LrtVaultDeposit {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        owner: fmt_addr(&ev.owner),
                        assets: ev.assets.to_string(),
                        shares: ev.shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lrt_vault::events::Withdraw::match_and_decode(log)
                {
                    events.lrt_vault_withdraws.push(LrtVaultWithdraw {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        receiver: fmt_addr(&ev.receiver),
                        owner: fmt_addr(&ev.owner),
                        assets: ev.assets.to_string(),
                        shares: ev.shares.to_string(),
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

    for e in events.lrt_vault_deposits {
        tables
            .create_row("lrt_vault_deposit", &e.id)
            .set("sender", &e.sender)
            .set("owner", &e.owner)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lrt_vault_withdraws {
        tables
            .create_row("lrt_vault_withdraw", &e.id)
            .set("sender", &e.sender)
            .set("receiver", &e.receiver)
            .set("owner", &e.owner)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
