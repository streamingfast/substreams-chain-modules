mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::stake_link_liquid::types::v1::{
    Events, PriorityPoolDeposit, PriorityPoolDepositTokens, PriorityPoolUnqueueTokens, PriorityPoolWithdraw, StlinkTransfer, StlinkUpdateStrategyRewards,
};

const STLINK: [u8; 20] = hex_literal::hex!("b8b295df2cd735b15be5eb419517aa626fc43cd5");
const PRIORITY_POOL: [u8; 20] = hex_literal::hex!("ddc796a66e8b83d0bccd97df33a6ccfba8fd60ea");

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

            if log.address() == STLINK.as_slice() {
                if let Some(ev) =
                    abi::stlink::events::Transfer::match_and_decode(log)
                {
                    events.stlink_transfers.push(StlinkTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        value: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::stlink::events::UpdateStrategyRewards::match_and_decode(log)
                {
                    events.stlink_update_strategy_rewardss.push(StlinkUpdateStrategyRewards {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        total_staked: ev.total_staked.to_string(),
                        rewards_amount: ev.rewards_amount.to_string(),
                        total_fees: ev.total_fees.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == PRIORITY_POOL.as_slice() {
                if let Some(ev) =
                    abi::priority_pool::events::Deposit::match_and_decode(log)
                {
                    events.priority_pool_deposits.push(PriorityPoolDeposit {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        pool_amount: ev.pool_amount.to_string(),
                        queue_amount: ev.queue_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::priority_pool::events::DepositTokens::match_and_decode(log)
                {
                    events.priority_pool_deposit_tokenss.push(PriorityPoolDepositTokens {
                        id: id.clone(),
                        unused_tokens_amount: ev.unused_tokens_amount.to_string(),
                        queued_tokens_amount: ev.queued_tokens_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::priority_pool::events::UnqueueTokens::match_and_decode(log)
                {
                    events.priority_pool_unqueue_tokenss.push(PriorityPoolUnqueueTokens {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::priority_pool::events::Withdraw::match_and_decode(log)
                {
                    events.priority_pool_withdraws.push(PriorityPoolWithdraw {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        amount: ev.amount.to_string(),
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

    for e in events.stlink_transfers {
        tables
            .create_row("stlink_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stlink_update_strategy_rewardss {
        tables
            .create_row("stlink_update_strategy_rewards", &e.id)
            .set("account", &e.account)
            .set("total_staked", &e.total_staked)
            .set("rewards_amount", &e.rewards_amount)
            .set("total_fees", &e.total_fees)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.priority_pool_deposits {
        tables
            .create_row("priority_pool_deposit", &e.id)
            .set("account", &e.account)
            .set("pool_amount", &e.pool_amount)
            .set("queue_amount", &e.queue_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.priority_pool_deposit_tokenss {
        tables
            .create_row("priority_pool_deposit_tokens", &e.id)
            .set("unused_tokens_amount", &e.unused_tokens_amount)
            .set("queued_tokens_amount", &e.queued_tokens_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.priority_pool_unqueue_tokenss {
        tables
            .create_row("priority_pool_unqueue_tokens", &e.id)
            .set("account", &e.account)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.priority_pool_withdraws {
        tables
            .create_row("priority_pool_withdraw", &e.id)
            .set("account", &e.account)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
