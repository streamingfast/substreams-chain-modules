mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::swell::types::v1::{
    Events, DepositManagerEthReceived, SwethNodeOperatorRewardPercentageUpdate, SwethReprice, SwethSwellTreasuryRewardPercentageUpdate, SwexitWithdrawRequestCreated, SwexitWithdrawalClaimed,
};

const DEPOSIT_MANAGER: [u8; 20] = hex_literal::hex!("b3d9cf8e163bbc840195a97e81f8a34e295b8f39");
const SWEXIT: [u8; 20] = hex_literal::hex!("48c11b86807627af70a34662d4865cf854251663");
const SWETH: [u8; 20] = hex_literal::hex!("f951e335afb289353dc249e82926178eac7ded78");

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

            if log.address == DEPOSIT_MANAGER {
                if let Some(ev) =
                    abi::deposit_manager::events::EthReceived::match_and_decode(log)
                {
                    events.deposit_manager_eth_receiveds.push(DepositManagerEthReceived {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == SWEXIT {
                if let Some(ev) =
                    abi::swexit::events::WithdrawRequestCreated::match_and_decode(log)
                {
                    events.swexit_withdraw_request_createds.push(SwexitWithdrawRequestCreated {
                        id: id.clone(),
                        token_id: ev.token_id.to_string(),
                        amount: ev.amount.to_string(),
                        evt_timestamp: ev.timestamp.to_string(),
                        last_token_id_processed: ev.last_token_id_processed.to_string(),
                        rate_when_created: ev.rate_when_created.to_string(),
                        owner: fmt_addr(&ev.owner),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::swexit::events::WithdrawalClaimed::match_and_decode(log)
                {
                    events.swexit_withdrawal_claimeds.push(SwexitWithdrawalClaimed {
                        id: id.clone(),
                        owner: fmt_addr(&ev.owner),
                        token_id: ev.token_id.to_string(),
                        exit_claimed_eth: ev.exit_claimed_eth.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == SWETH {
                if let Some(ev) =
                    abi::sweth::events::Reprice::match_and_decode(log)
                {
                    events.sweth_reprices.push(SwethReprice {
                        id: id.clone(),
                        new_eth_reserves: ev.new_eth_reserves.to_string(),
                        new_sw_eth_to_eth_rate: ev.new_sw_eth_to_eth_rate.to_string(),
                        node_operator_rewards: ev.node_operator_rewards.to_string(),
                        swell_treasury_rewards: ev.swell_treasury_rewards.to_string(),
                        total_eth_deposited: ev.total_eth_deposited.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::sweth::events::NodeOperatorRewardPercentageUpdate::match_and_decode(log)
                {
                    events.sweth_node_operator_reward_percentage_updates.push(SwethNodeOperatorRewardPercentageUpdate {
                        id: id.clone(),
                        old_percentage: ev.old_percentage.to_string(),
                        new_percentage: ev.new_percentage.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::sweth::events::SwellTreasuryRewardPercentageUpdate::match_and_decode(log)
                {
                    events.sweth_swell_treasury_reward_percentage_updates.push(SwethSwellTreasuryRewardPercentageUpdate {
                        id: id.clone(),
                        old_percentage: ev.old_percentage.to_string(),
                        new_percentage: ev.new_percentage.to_string(),
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

    for e in events.deposit_manager_eth_receiveds {
        tables
            .create_row("deposit_manager_eth_received", &e.id)
            .set("from", &e.from)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.swexit_withdraw_request_createds {
        tables
            .create_row("swexit_withdraw_request_created", &e.id)
            .set("token_id", &e.token_id)
            .set("amount", &e.amount)
            .set("evt_timestamp", &e.evt_timestamp)
            .set("last_token_id_processed", &e.last_token_id_processed)
            .set("rate_when_created", &e.rate_when_created)
            .set("owner", &e.owner)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.swexit_withdrawal_claimeds {
        tables
            .create_row("swexit_withdrawal_claimed", &e.id)
            .set("owner", &e.owner)
            .set("token_id", &e.token_id)
            .set("exit_claimed_eth", &e.exit_claimed_eth)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.sweth_reprices {
        tables
            .create_row("sweth_reprice", &e.id)
            .set("new_eth_reserves", &e.new_eth_reserves)
            .set("new_sw_eth_to_eth_rate", &e.new_sw_eth_to_eth_rate)
            .set("node_operator_rewards", &e.node_operator_rewards)
            .set("swell_treasury_rewards", &e.swell_treasury_rewards)
            .set("total_eth_deposited", &e.total_eth_deposited)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.sweth_node_operator_reward_percentage_updates {
        tables
            .create_row("sweth_node_operator_reward_percentage_update", &e.id)
            .set("old_percentage", &e.old_percentage)
            .set("new_percentage", &e.new_percentage)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.sweth_swell_treasury_reward_percentage_updates {
        tables
            .create_row("sweth_swell_treasury_reward_percentage_update", &e.id)
            .set("old_percentage", &e.old_percentage)
            .set("new_percentage", &e.new_percentage)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
