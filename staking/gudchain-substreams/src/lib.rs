mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::gudchain::types::v1::{
    Events, VaultV1Deposit, VaultV1Withdraw,
};

const VAULT_V1: [u8; 20] = hex_literal::hex!("d759e176def0f14e5c2d300238d41b1cbb5585bf");

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

            if log.address() == VAULT_V1.as_slice() {
                if let Some(ev) =
                    abi::vault_v1::events::Deposit::match_and_decode(log)
                {
                    events.vault_v1_deposits.push(VaultV1Deposit {
                        id: id.clone(),
                        token: fmt_addr(&ev.token),
                        user: fmt_addr(&ev.user),
                        deposit_amt: ev.deposit_amt.to_string(),
                        share_issued: ev.share_issued.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::vault_v1::events::Withdraw::match_and_decode(log)
                {
                    events.vault_v1_withdraws.push(VaultV1Withdraw {
                        id: id.clone(),
                        token: fmt_addr(&ev.token),
                        user: fmt_addr(&ev.user),
                        amount: ev.amount.to_string(),
                        share_burnt: ev.share_burnt.to_string(),
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

    for e in events.vault_v1_deposits {
        tables
            .create_row("vault_v1_deposit", &e.id)
            .set("token", &e.token)
            .set("user", &e.user)
            .set("deposit_amt", &e.deposit_amt)
            .set("share_issued", &e.share_issued)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.vault_v1_withdraws {
        tables
            .create_row("vault_v1_withdraw", &e.id)
            .set("token", &e.token)
            .set("user", &e.user)
            .set("amount", &e.amount)
            .set("share_burnt", &e.share_burnt)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
