mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::node_dao::types::v1::{
    Events, LiquidStakingEthStake, LiquidStakingEthUnstake, NethPoolEthStake, NethPoolEthUnstake, NethPoolWithdrawalsClaimed, NethPoolWithdrawalsRequest, RestakingPoolEthStake, RestakingPoolEthUnstake, RestakingPoolWithdrawalsClaimed, RestakingPoolWithdrawalsRequest, WithdrawalRequestLargeWithdrawalsClaim, WithdrawalRequestLargeWithdrawalsRequest, WithdrawalRequestWithdrawalsReceive,
};

const LIQUID_STAKING: [u8; 20] = hex_literal::hex!("8103151e2377e78c04a3d2564e20542680ed3096");
const WITHDRAWAL_REQUEST: [u8; 20] = hex_literal::hex!("e81fc969d14cad8537ebafa2a1c478f29d7840fc");
const NETH_POOL: [u8; 20] = hex_literal::hex!("f3c79408164abfb6fd5ddfe33b084e4ad2c07c18");
const RESTAKING_POOL: [u8; 20] = hex_literal::hex!("0d6f764452ca43eb8bd22788c9db43e4b5a725bc");

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

            if log.address == LIQUID_STAKING {
                if let Some(ev) =
                    abi::liquid_staking::events::EthStake::match_and_decode(log)
                {
                    events.liquid_staking_eth_stakes.push(LiquidStakingEthStake {
                        id: id.clone(),
                        operator_id: ev.operator_id.to_string(),
                        from: fmt_addr(&ev.from),
                        amount: ev.amount.to_string(),
                        amount_out: ev.amount_out.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::liquid_staking::events::EthUnstake::match_and_decode(log)
                {
                    events.liquid_staking_eth_unstakes.push(LiquidStakingEthUnstake {
                        id: id.clone(),
                        operator_id: ev.operator_id.to_string(),
                        target_operator_id: ev.target_operator_id.to_string(),
                        ender: fmt_addr(&ev.ender),
                        amounts: ev.amounts.to_string(),
                        amount_out: ev.amount_out.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == WITHDRAWAL_REQUEST {
                if let Some(ev) =
                    abi::withdrawal_request::events::WithdrawalsReceive::match_and_decode(log)
                {
                    events.withdrawal_request_withdrawals_receives.push(WithdrawalRequestWithdrawalsReceive {
                        id: id.clone(),
                        operator_id: ev.operator_id.to_string(),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::withdrawal_request::events::LargeWithdrawalsRequest::match_and_decode(log)
                {
                    events.withdrawal_request_large_withdrawals_requests.push(WithdrawalRequestLargeWithdrawalsRequest {
                        id: id.clone(),
                        operator_id: ev.operator_id.to_string(),
                        sender: fmt_addr(&ev.sender),
                        total_neth_amount: ev.total_neth_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::withdrawal_request::events::LargeWithdrawalsClaim::match_and_decode(log)
                {
                    events.withdrawal_request_large_withdrawals_claims.push(WithdrawalRequestLargeWithdrawalsClaim {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        total_pending_eth_amount: ev.total_pending_eth_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == NETH_POOL {
                if let Some(ev) =
                    abi::neth_pool::events::EthStake::match_and_decode(log)
                {
                    events.neth_pool_eth_stakes.push(NethPoolEthStake {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        stake_amount: ev.stake_amount.to_string(),
                        mint_amount: ev.mint_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::neth_pool::events::EthUnstake::match_and_decode(log)
                {
                    events.neth_pool_eth_unstakes.push(NethPoolEthUnstake {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        unstake_amount: ev.unstake_amount.to_string(),
                        eth_amount: ev.eth_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::neth_pool::events::WithdrawalsRequest::match_and_decode(log)
                {
                    events.neth_pool_withdrawals_requests.push(NethPoolWithdrawalsRequest {
                        id: id.clone(),
                        receiver: fmt_addr(&ev.receiver),
                        withdrawal_amount: ev.withdrawal_amount.to_string(),
                        block_number: ev.block_number.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::neth_pool::events::WithdrawalsClaimed::match_and_decode(log)
                {
                    events.neth_pool_withdrawals_claimeds.push(NethPoolWithdrawalsClaimed {
                        id: id.clone(),
                        receiver: fmt_addr(&ev.receiver),
                        request_id: ev.request_id.to_string(),
                        claim_amount: ev.claim_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == RESTAKING_POOL {
                if let Some(ev) =
                    abi::restaking_pool::events::EthStake::match_and_decode(log)
                {
                    events.restaking_pool_eth_stakes.push(RestakingPoolEthStake {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        stake_amount: ev.stake_amount.to_string(),
                        mint_amount: ev.mint_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::restaking_pool::events::EthUnstake::match_and_decode(log)
                {
                    events.restaking_pool_eth_unstakes.push(RestakingPoolEthUnstake {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        unstake_amount: ev.unstake_amount.to_string(),
                        eth_amount: ev.eth_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::restaking_pool::events::WithdrawalsRequest::match_and_decode(log)
                {
                    events.restaking_pool_withdrawals_requests.push(RestakingPoolWithdrawalsRequest {
                        id: id.clone(),
                        receiver: fmt_addr(&ev.receiver),
                        withdrawal_amount: ev.withdrawal_amount.to_string(),
                        block_number: ev.block_number.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::restaking_pool::events::WithdrawalsClaimed::match_and_decode(log)
                {
                    events.restaking_pool_withdrawals_claimeds.push(RestakingPoolWithdrawalsClaimed {
                        id: id.clone(),
                        receiver: fmt_addr(&ev.receiver),
                        request_id: ev.request_id.to_string(),
                        claim_amount: ev.claim_amount.to_string(),
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

    for e in events.liquid_staking_eth_stakes {
        tables
            .create_row("liquid_staking_eth_stake", &e.id)
            .set("operator_id", &e.operator_id)
            .set("from", &e.from)
            .set("amount", &e.amount)
            .set("amount_out", &e.amount_out)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.liquid_staking_eth_unstakes {
        tables
            .create_row("liquid_staking_eth_unstake", &e.id)
            .set("operator_id", &e.operator_id)
            .set("target_operator_id", &e.target_operator_id)
            .set("ender", &e.ender)
            .set("amounts", &e.amounts)
            .set("amount_out", &e.amount_out)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawal_request_withdrawals_receives {
        tables
            .create_row("withdrawal_request_withdrawals_receive", &e.id)
            .set("operator_id", &e.operator_id)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawal_request_large_withdrawals_requests {
        tables
            .create_row("withdrawal_request_large_withdrawals_request", &e.id)
            .set("operator_id", &e.operator_id)
            .set("sender", &e.sender)
            .set("total_neth_amount", &e.total_neth_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdrawal_request_large_withdrawals_claims {
        tables
            .create_row("withdrawal_request_large_withdrawals_claim", &e.id)
            .set("sender", &e.sender)
            .set("total_pending_eth_amount", &e.total_pending_eth_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.neth_pool_eth_stakes {
        tables
            .create_row("neth_pool_eth_stake", &e.id)
            .set("staker", &e.staker)
            .set("stake_amount", &e.stake_amount)
            .set("mint_amount", &e.mint_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.neth_pool_eth_unstakes {
        tables
            .create_row("neth_pool_eth_unstake", &e.id)
            .set("sender", &e.sender)
            .set("unstake_amount", &e.unstake_amount)
            .set("eth_amount", &e.eth_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.neth_pool_withdrawals_requests {
        tables
            .create_row("neth_pool_withdrawals_request", &e.id)
            .set("receiver", &e.receiver)
            .set("withdrawal_amount", &e.withdrawal_amount)
            .set("block_number", &e.block_number)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.neth_pool_withdrawals_claimeds {
        tables
            .create_row("neth_pool_withdrawals_claimed", &e.id)
            .set("receiver", &e.receiver)
            .set("request_id", &e.request_id)
            .set("claim_amount", &e.claim_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.restaking_pool_eth_stakes {
        tables
            .create_row("restaking_pool_eth_stake", &e.id)
            .set("staker", &e.staker)
            .set("stake_amount", &e.stake_amount)
            .set("mint_amount", &e.mint_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.restaking_pool_eth_unstakes {
        tables
            .create_row("restaking_pool_eth_unstake", &e.id)
            .set("sender", &e.sender)
            .set("unstake_amount", &e.unstake_amount)
            .set("eth_amount", &e.eth_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.restaking_pool_withdrawals_requests {
        tables
            .create_row("restaking_pool_withdrawals_request", &e.id)
            .set("receiver", &e.receiver)
            .set("withdrawal_amount", &e.withdrawal_amount)
            .set("block_number", &e.block_number)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.restaking_pool_withdrawals_claimeds {
        tables
            .create_row("restaking_pool_withdrawals_claimed", &e.id)
            .set("receiver", &e.receiver)
            .set("request_id", &e.request_id)
            .set("claim_amount", &e.claim_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
