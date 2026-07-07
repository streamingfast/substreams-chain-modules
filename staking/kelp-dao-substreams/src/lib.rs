mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::kelp::types::v1::{
    AssetDeposit, AssetWithdrawalFinalized, AssetWithdrawalQueued, EthDeposit, Events,
};

const DEPOSIT_POOL: [u8; 20] = hex_literal::hex!("036676389e48133b63a802f8635ad39e752d375d");
const WITHDRAWAL_MANAGER: [u8; 20] =
    hex_literal::hex!("62de59c08eb5dae4b7e6f7a8cad3006d6965ec16");

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

            if log.address == DEPOSIT_POOL {
                if let Some(ev) =
                    abi::lrt_deposit_pool::events::AssetDeposit::match_and_decode(log)
                {
                    events.asset_deposits.push(AssetDeposit {
                        id,
                        depositor: fmt_addr(&ev.depositor),
                        asset: fmt_addr(&ev.asset),
                        deposit_amount: ev.deposit_amount.to_string(),
                        rseth_mint_amount: ev.rseth_mint_amount.to_string(),
                        referral_id: ev.referral_id,
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::lrt_deposit_pool::events::EthDeposit::match_and_decode(log)
                {
                    events.eth_deposits.push(EthDeposit {
                        id,
                        depositor: fmt_addr(&ev.depositor),
                        deposit_amount: ev.deposit_amount.to_string(),
                        rseth_mint_amount: ev.rseth_mint_amount.to_string(),
                        referral_id: ev.referral_id,
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == WITHDRAWAL_MANAGER {
                if let Some(ev) =
                    abi::lrt_withdrawal_manager::events::AssetWithdrawalQueued::match_and_decode(
                        log,
                    )
                {
                    events.withdrawals_queued.push(AssetWithdrawalQueued {
                        id,
                        withdrawer: fmt_addr(&ev.withdrawer),
                        asset: fmt_addr(&ev.asset),
                        rseth_unstaked: ev.rs_eth_unstaked.to_string(),
                        user_nonce: ev.user_nonce.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::lrt_withdrawal_manager::events::AssetWithdrawalFinalized::match_and_decode(
                        log,
                    )
                {
                    events.withdrawals_finalized.push(AssetWithdrawalFinalized {
                        id,
                        withdrawer: fmt_addr(&ev.withdrawer),
                        asset: fmt_addr(&ev.asset),
                        amount_burned: ev.amount_burned.to_string(),
                        amount_received: ev.amount_received.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.asset_deposits {
        tables
            .create_row("asset_deposits", &e.id)
            .set("depositor", &e.depositor)
            .set("asset", &e.asset)
            .set("deposit_amount", &e.deposit_amount)
            .set("rseth_mint_amount", &e.rseth_mint_amount)
            .set("referral_id", &e.referral_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.eth_deposits {
        tables
            .create_row("eth_deposits", &e.id)
            .set("depositor", &e.depositor)
            .set("deposit_amount", &e.deposit_amount)
            .set("rseth_mint_amount", &e.rseth_mint_amount)
            .set("referral_id", &e.referral_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawals_queued {
        tables
            .create_row("withdrawals_queued", &e.id)
            .set("withdrawer", &e.withdrawer)
            .set("asset", &e.asset)
            .set("rseth_unstaked", &e.rseth_unstaked)
            .set("user_nonce", &e.user_nonce)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawals_finalized {
        tables
            .create_row("withdrawals_finalized", &e.id)
            .set("withdrawer", &e.withdrawer)
            .set("asset", &e.asset)
            .set("amount_burned", &e.amount_burned)
            .set("amount_received", &e.amount_received)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
