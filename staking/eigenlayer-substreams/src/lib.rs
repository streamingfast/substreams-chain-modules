mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::eigenlayer::types::v1::{
    BeaconChainEthDeposited, Deposit, Events, PodDeployed, ShareWithdrawalQueued,
    WithdrawalCompleted, WithdrawalQueued,
};

const STRATEGY_MANAGER: [u8; 20] =
    hex_literal::hex!("858646372cc42e1a627fce94aa7a7033e7cf075a");

const EIGEN_POD_MANAGER: [u8; 20] =
    hex_literal::hex!("91e677b07f7af907ec9a428aafa9fc14a0d3a338");

fn block_timestamp(block: &Block) -> u64 {
    block
        .header
        .as_ref()
        .and_then(|h| h.timestamp.as_ref().map(|t| t.seconds as u64))
        .unwrap_or(0)
}

fn fmt_addr(addr: &[u8]) -> String {
    format!("0x{}", hex::encode(addr))
}

fn fmt_bytes32(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for (log, _call) in trx.logs_with_calls() {
            let id = format!("{}-{}", tx_hash, log.index);

            if log.address == STRATEGY_MANAGER {
                if let Some(ev) =
                    abi::strategy_manager::events::Deposit::match_and_decode(log)
                {
                    events.deposits.push(Deposit {
                        id,
                        depositor: fmt_addr(&ev.depositor),
                        token: fmt_addr(&ev.token),
                        strategy: fmt_addr(&ev.strategy),
                        shares: ev.shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::strategy_manager::events::WithdrawalQueued::match_and_decode(log)
                {
                    events.withdrawals_queued.push(WithdrawalQueued {
                        id,
                        depositor: fmt_addr(&ev.depositor),
                        nonce: ev.nonce.to_string(),
                        withdrawer: fmt_addr(&ev.withdrawer),
                        delegated_address: fmt_addr(&ev.delegated_address),
                        withdrawal_root: fmt_bytes32(&ev.withdrawal_root),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::strategy_manager::events::WithdrawalCompleted::match_and_decode(log)
                {
                    events.withdrawals_completed.push(WithdrawalCompleted {
                        id,
                        depositor: fmt_addr(&ev.depositor),
                        nonce: ev.nonce.to_string(),
                        withdrawer: fmt_addr(&ev.withdrawer),
                        withdrawal_root: fmt_bytes32(&ev.withdrawal_root),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::strategy_manager::events::ShareWithdrawalQueued::match_and_decode(log)
                {
                    events.share_withdrawals_queued.push(ShareWithdrawalQueued {
                        id,
                        depositor: fmt_addr(&ev.depositor),
                        nonce: ev.nonce.to_string(),
                        strategy: fmt_addr(&ev.strategy),
                        shares: ev.shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if log.address == EIGEN_POD_MANAGER {
                if let Some(ev) =
                    abi::eigen_pod_manager::events::BeaconChainEthDeposited::match_and_decode(log)
                {
                    events.beacon_chain_eth_deposits.push(BeaconChainEthDeposited {
                        id,
                        pod_owner: fmt_addr(&ev.pod_owner),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::eigen_pod_manager::events::PodDeployed::match_and_decode(log)
                {
                    events.pods_deployed.push(PodDeployed {
                        id,
                        eigen_pod: fmt_addr(&ev.eigen_pod),
                        pod_owner: fmt_addr(&ev.pod_owner),
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

    for e in events.deposits {
        tables
            .create_row("deposits", &e.id)
            .set("depositor", &e.depositor)
            .set("token", &e.token)
            .set("strategy", &e.strategy)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawals_queued {
        tables
            .create_row("withdrawals_queued", &e.id)
            .set("depositor", &e.depositor)
            .set("nonce", &e.nonce)
            .set("withdrawer", &e.withdrawer)
            .set("delegated_address", &e.delegated_address)
            .set("withdrawal_root", &e.withdrawal_root)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawals_completed {
        tables
            .create_row("withdrawals_completed", &e.id)
            .set("depositor", &e.depositor)
            .set("nonce", &e.nonce)
            .set("withdrawer", &e.withdrawer)
            .set("withdrawal_root", &e.withdrawal_root)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.share_withdrawals_queued {
        tables
            .create_row("share_withdrawals_queued", &e.id)
            .set("depositor", &e.depositor)
            .set("nonce", &e.nonce)
            .set("strategy", &e.strategy)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.beacon_chain_eth_deposits {
        tables
            .create_row("beacon_chain_eth_deposits", &e.id)
            .set("pod_owner", &e.pod_owner)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pods_deployed {
        tables
            .create_row("pods_deployed", &e.id)
            .set("eigen_pod", &e.eigen_pod)
            .set("pod_owner", &e.pod_owner)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
