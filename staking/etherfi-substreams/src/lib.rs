mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::etherfi::types::v1::{
    Deposit, EarlyDeposit, EarlyWithdrawn, Events, FullWithdrawal, FundsClaimed, NodeEvicted,
    NodeExitProcessed, NodeExitRequested, PartialWithdrawal, Rebase, ValidatorApproved,
    ValidatorRegistered, Withdraw,
};

const EARLY_ADOPTER_POOL: [u8; 20] =
    hex_literal::hex!("7623e9dc0da6ff821ddb9ebaba794054e078f8c4");

const LIQUIDITY_POOL: [u8; 20] =
    hex_literal::hex!("308861a430be4cce5502d0a12724771fc6daf216");

const NODES_MANAGER: [u8; 20] =
    hex_literal::hex!("8b71140ad2e5d1e7018d2a7f8a288bd3cd38916f");

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

            if log.address == EARLY_ADOPTER_POOL {
                if let Some(ev) =
                    abi::early_adopter_pool::events::DepositEth::match_and_decode(log)
                {
                    events.early_deposits.push(EarlyDeposit {
                        id,
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
                        token: "ETH".to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::early_adopter_pool::events::DepositErc20::match_and_decode(log)
                {
                    events.early_deposits.push(EarlyDeposit {
                        id,
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
                        token: "ERC20".to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::early_adopter_pool::events::Withdrawn::match_and_decode(log)
                {
                    events.early_withdrawns.push(EarlyWithdrawn {
                        id,
                        sender: fmt_addr(&ev.sender),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::early_adopter_pool::events::Fundsclaimed::match_and_decode(log)
                {
                    events.funds_claimed.push(FundsClaimed {
                        id,
                        user: fmt_addr(&ev.user),
                        points_accumulated: ev.points_accumulated.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if log.address == LIQUIDITY_POOL {
                if let Some(ev) =
                    abi::liquidity_pool::events::Deposit1::match_and_decode(log)
                {
                    events.deposits.push(Deposit {
                        id,
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
                        source: String::new(),
                        referral: String::new(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::liquidity_pool::events::Deposit2::match_and_decode(log)
                {
                    events.deposits.push(Deposit {
                        id,
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
                        source: ev.source.to_string(),
                        referral: fmt_addr(&ev.referral),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::liquidity_pool::events::Withdraw1::match_and_decode(log)
                {
                    events.withdraws.push(Withdraw {
                        id,
                        sender: fmt_addr(&ev.sender),
                        recipient: fmt_addr(&ev.recipient),
                        amount: ev.amount.to_string(),
                        source: String::new(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::liquidity_pool::events::Withdraw2::match_and_decode(log)
                {
                    events.withdraws.push(Withdraw {
                        id,
                        sender: fmt_addr(&ev.sender),
                        recipient: fmt_addr(&ev.recipient),
                        amount: ev.amount.to_string(),
                        source: ev.source.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::liquidity_pool::events::Rebase::match_and_decode(log)
                {
                    events.rebases.push(Rebase {
                        id,
                        total_eth_locked: ev.total_eth_locked.to_string(),
                        total_eeth_shares: ev.total_e_eth_shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::liquidity_pool::events::ValidatorRegistered::match_and_decode(log)
                {
                    events.validators_registered.push(ValidatorRegistered {
                        id,
                        validator_id: ev.validator_id.to_string(),
                        pub_key: format!("0x{}", hex::encode(&ev.pub_key)),
                        deposit_root: fmt_bytes32(&ev.deposit_root),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::liquidity_pool::events::ValidatorApproved::match_and_decode(log)
                {
                    events.validators_approved.push(ValidatorApproved {
                        id,
                        validator_id: ev.validator_id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if log.address == NODES_MANAGER {
                if let Some(ev) =
                    abi::etherfi_nodes_manager::events::FullWithdrawal::match_and_decode(log)
                {
                    events.full_withdrawals.push(FullWithdrawal {
                        id,
                        validator_id: ev.validator_id.to_string(),
                        etherfi_node: fmt_addr(&ev.ether_fi_node),
                        to_operator: ev.to_operator.to_string(),
                        to_tnft: ev.to_tnft.to_string(),
                        to_bnft: ev.to_bnft.to_string(),
                        to_treasury: ev.to_treasury.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::etherfi_nodes_manager::events::PartialWithdrawal::match_and_decode(log)
                {
                    events.partial_withdrawals.push(PartialWithdrawal {
                        id,
                        validator_id: ev.validator_id.to_string(),
                        etherfi_node: fmt_addr(&ev.ether_fi_node),
                        to_operator: ev.to_operator.to_string(),
                        to_tnft: ev.to_tnft.to_string(),
                        to_bnft: ev.to_bnft.to_string(),
                        to_treasury: ev.to_treasury.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::etherfi_nodes_manager::events::NodeExitRequested::match_and_decode(log)
                {
                    events.node_exit_requests.push(NodeExitRequested {
                        id,
                        validator_id: ev.validator_id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::etherfi_nodes_manager::events::NodeExitProcessed::match_and_decode(log)
                {
                    events.node_exit_processed.push(NodeExitProcessed {
                        id,
                        validator_id: ev.validator_id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::etherfi_nodes_manager::events::NodeEvicted::match_and_decode(log)
                {
                    events.node_evictions.push(NodeEvicted {
                        id,
                        validator_id: ev.validator_id.to_string(),
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

    for e in events.early_deposits {
        tables
            .create_row("early_deposits", &e.id)
            .set("sender", &e.sender)
            .set("amount", &e.amount)
            .set("token", &e.token)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.early_withdrawns {
        tables
            .create_row("early_withdrawns", &e.id)
            .set("sender", &e.sender)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.funds_claimed {
        tables
            .create_row("funds_claimed", &e.id)
            .set("user", &e.user)
            .set("points_accumulated", &e.points_accumulated)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.deposits {
        tables
            .create_row("deposits", &e.id)
            .set("sender", &e.sender)
            .set("amount", &e.amount)
            .set("source", &e.source)
            .set("referral", &e.referral)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdraws {
        tables
            .create_row("withdraws", &e.id)
            .set("sender", &e.sender)
            .set("recipient", &e.recipient)
            .set("amount", &e.amount)
            .set("source", &e.source)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rebases {
        tables
            .create_row("rebases", &e.id)
            .set("total_eth_locked", &e.total_eth_locked)
            .set("total_eeth_shares", &e.total_eeth_shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.validators_registered {
        tables
            .create_row("validators_registered", &e.id)
            .set("validator_id", &e.validator_id)
            .set("pub_key", &e.pub_key)
            .set("deposit_root", &e.deposit_root)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.validators_approved {
        tables
            .create_row("validators_approved", &e.id)
            .set("validator_id", &e.validator_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.full_withdrawals {
        tables
            .create_row("full_withdrawals", &e.id)
            .set("validator_id", &e.validator_id)
            .set("etherfi_node", &e.etherfi_node)
            .set("to_operator", &e.to_operator)
            .set("to_tnft", &e.to_tnft)
            .set("to_bnft", &e.to_bnft)
            .set("to_treasury", &e.to_treasury)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.partial_withdrawals {
        tables
            .create_row("partial_withdrawals", &e.id)
            .set("validator_id", &e.validator_id)
            .set("etherfi_node", &e.etherfi_node)
            .set("to_operator", &e.to_operator)
            .set("to_tnft", &e.to_tnft)
            .set("to_bnft", &e.to_bnft)
            .set("to_treasury", &e.to_treasury)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.node_exit_requests {
        tables
            .create_row("node_exit_requests", &e.id)
            .set("validator_id", &e.validator_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.node_exit_processed {
        tables
            .create_row("node_exit_processed", &e.id)
            .set("validator_id", &e.validator_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.node_evictions {
        tables
            .create_row("node_evictions", &e.id)
            .set("validator_id", &e.validator_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
