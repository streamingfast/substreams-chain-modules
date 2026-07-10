mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::bob_fusion::types::v1::{
    Events, FusionLockDeposit, FusionLockWithdrawToL1, FusionLockWithdrawToL2,
};

const FUSION_LOCK: [u8; 20] = hex_literal::hex!("61dc14b28d4dbcd6cf887e9b72018b9da1ce6ff7");

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

            if log.address == FUSION_LOCK {
                if let Some(ev) =
                    abi::fusion_lock::events::Deposit::match_and_decode(log)
                {
                    events.fusion_lock_deposits.push(FusionLockDeposit {
                        id: id.clone(),
                        deposit_owner: fmt_addr(&ev.deposit_owner),
                        token: fmt_addr(&ev.token),
                        amount: ev.amount.to_string(),
                        deposit_time: ev.deposit_time.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::fusion_lock::events::WithdrawToL1::match_and_decode(log)
                {
                    events.fusion_lock_withdraw_to_l1s.push(FusionLockWithdrawToL1 {
                        id: id.clone(),
                        owner: fmt_addr(&ev.owner),
                        token: fmt_addr(&ev.token),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::fusion_lock::events::WithdrawToL2::match_and_decode(log)
                {
                    events.fusion_lock_withdraw_to_l2s.push(FusionLockWithdrawToL2 {
                        id: id.clone(),
                        owner: fmt_addr(&ev.owner),
                        receiver: fmt_addr(&ev.receiver),
                        l1_token: fmt_addr(&ev.l1_token),
                        l2_token: fmt_addr(&ev.l2_token),
                        amount: ev.amount.to_string(),
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

    for e in events.fusion_lock_deposits {
        tables
            .create_row("fusion_lock_deposit", &e.id)
            .set("deposit_owner", &e.deposit_owner)
            .set("token", &e.token)
            .set("amount", &e.amount)
            .set("deposit_time", &e.deposit_time)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.fusion_lock_withdraw_to_l1s {
        tables
            .create_row("fusion_lock_withdraw_to_l1", &e.id)
            .set("owner", &e.owner)
            .set("token", &e.token)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.fusion_lock_withdraw_to_l2s {
        tables
            .create_row("fusion_lock_withdraw_to_l2", &e.id)
            .set("owner", &e.owner)
            .set("receiver", &e.receiver)
            .set("l1_token", &e.l1_token)
            .set("l2_token", &e.l2_token)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
