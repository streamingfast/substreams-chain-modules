mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::slisbnb::types::v1::{
    Events, ListaStakeManagerClaimWithdrawal, ListaStakeManagerDeposit, ListaStakeManagerRequestWithdraw, ListaStakeManagerRewardsCompounded,
};

const LISTA_STAKE_MANAGER: [u8; 20] = hex_literal::hex!("1adb950d8bb3da4be104211d5ab038628e477fe6");

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

            if log.address == LISTA_STAKE_MANAGER {
                if let Some(ev) =
                    abi::lista_stake_manager::events::Deposit::match_and_decode(log)
                {
                    events.lista_stake_manager_deposits.push(ListaStakeManagerDeposit {
                        id: id.clone(),
                        src: fmt_addr(&ev.src),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lista_stake_manager::events::RequestWithdraw::match_and_decode(log)
                {
                    events.lista_stake_manager_request_withdraws.push(ListaStakeManagerRequestWithdraw {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        amount_in_slis_bnb: ev.amount_in_slis_bnb.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lista_stake_manager::events::ClaimWithdrawal::match_and_decode(log)
                {
                    events.lista_stake_manager_claim_withdrawals.push(ListaStakeManagerClaimWithdrawal {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        idx: ev.idx.to_string(),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lista_stake_manager::events::RewardsCompounded::match_and_decode(log)
                {
                    events.lista_stake_manager_rewards_compoundeds.push(ListaStakeManagerRewardsCompounded {
                        id: id.clone(),
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

    for e in events.lista_stake_manager_deposits {
        tables
            .create_row("lista_stake_manager_deposit", &e.id)
            .set("src", &e.src)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lista_stake_manager_request_withdraws {
        tables
            .create_row("lista_stake_manager_request_withdraw", &e.id)
            .set("account", &e.account)
            .set("amount_in_slis_bnb", &e.amount_in_slis_bnb)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lista_stake_manager_claim_withdrawals {
        tables
            .create_row("lista_stake_manager_claim_withdrawal", &e.id)
            .set("account", &e.account)
            .set("idx", &e.idx)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lista_stake_manager_rewards_compoundeds {
        tables
            .create_row("lista_stake_manager_rewards_compounded", &e.id)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
