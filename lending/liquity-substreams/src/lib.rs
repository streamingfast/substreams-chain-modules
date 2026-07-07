mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::liquity::types::v1::{
    ActivePoolEthBalanceUpdated, ActivePoolLusdDebtUpdated, CollBalanceUpdated, EthGainWithdrawn,
    Events, LastGoodPriceUpdated, Liquidation, LusdBorrowingFeePaid, Redemption,
    StabilityPoolEthBalanceUpdated, StabilityPoolLusdBalanceUpdated, TroveLiquidated, TroveUpdated,
    UserDepositChanged,
};

const TROVE_MANAGER: [u8; 20] = hex_literal::hex!("a39739ef8b0231dbfa0dcda07d7e29faabcf4bb2");
const BORROWER_OPERATIONS: [u8; 20] =
    hex_literal::hex!("24179cd81c9e782a4096035f7ec97fb8b783e007");
const PRICE_FEED: [u8; 20] = hex_literal::hex!("4c517d4e2c851ca76d7ec94b805269df0f2201de");
const ACTIVE_POOL: [u8; 20] = hex_literal::hex!("df9eb223bafbe5c5271415c75aecd68c21fe3d7f");
const COLL_SURPLUS_POOL: [u8; 20] =
    hex_literal::hex!("3d32e8b97ed5881324241cf03b2da5e2ebce5521");
const STABILITY_POOL: [u8; 20] = hex_literal::hex!("66017d22b0f8556afdd19fc67041899eb65a21bb");

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

            if log.address == TROVE_MANAGER {
                if let Some(ev) =
                    abi::trove_manager::events::TroveUpdated::match_and_decode(log)
                {
                    events.trove_updated.push(TroveUpdated {
                        id: id.clone(),
                        borrower: fmt_addr(&ev.borrower),
                        debt: ev.debt.to_string(),
                        coll: ev.coll.to_string(),
                        stake: ev.stake.to_string(),
                        operation: ev.operation.to_string(),
                        source: "TroveManager".to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::trove_manager::events::TroveLiquidated::match_and_decode(log)
                {
                    events.trove_liquidated.push(TroveLiquidated {
                        id: id.clone(),
                        borrower: fmt_addr(&ev.borrower),
                        debt: ev.debt.to_string(),
                        coll: ev.coll.to_string(),
                        operation: ev.operation.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) = abi::trove_manager::events::Liquidation::match_and_decode(log) {
                    events.liquidations.push(Liquidation {
                        id: id.clone(),
                        liquidated_debt: ev.liquidated_debt.to_string(),
                        liquidated_coll: ev.liquidated_coll.to_string(),
                        coll_gas_compensation: ev.coll_gas_compensation.to_string(),
                        lusd_gas_compensation: ev.lusd_gas_compensation.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) = abi::trove_manager::events::Redemption::match_and_decode(log) {
                    events.redemptions.push(Redemption {
                        id: id.clone(),
                        attempted_lusd_amount: ev.attempted_lusd_amount.to_string(),
                        actual_lusd_amount: ev.actual_lusd_amount.to_string(),
                        eth_sent: ev.eth_sent.to_string(),
                        eth_fee: ev.eth_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                continue;
            }

            if log.address == BORROWER_OPERATIONS {
                if let Some(ev) =
                    abi::borrower_operations::events::LusdBorrowingFeePaid::match_and_decode(log)
                {
                    events.lusd_borrowing_fee_paid.push(LusdBorrowingFeePaid {
                        id: id.clone(),
                        borrower: fmt_addr(&ev.borrower),
                        lusd_fee: ev.lusd_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::borrower_operations::events::TroveUpdated::match_and_decode(log)
                {
                    events.trove_updated.push(TroveUpdated {
                        id: id.clone(),
                        borrower: fmt_addr(&ev.borrower),
                        debt: ev.debt.to_string(),
                        coll: ev.coll.to_string(),
                        stake: ev.stake.to_string(),
                        operation: ev.operation.to_string(),
                        source: "BorrowerOperations".to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                continue;
            }

            if log.address == PRICE_FEED {
                if let Some(ev) =
                    abi::price_feed::events::LastGoodPriceUpdated::match_and_decode(log)
                {
                    events.last_good_price_updated.push(LastGoodPriceUpdated {
                        id: id.clone(),
                        price: ev.last_good_price.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if log.address == ACTIVE_POOL {
                if let Some(ev) =
                    abi::active_pool::events::ActivePoolEthBalanceUpdated::match_and_decode(log)
                {
                    events
                        .active_pool_eth_balance_updated
                        .push(ActivePoolEthBalanceUpdated {
                            id: id.clone(),
                            eth: ev.eth.to_string(),
                            tx_hash: tx_hash.clone(),
                            log_index: log.index as u64,
                            block_num: block.number,
                            timestamp,
                        });
                    continue;
                }
                if let Some(ev) =
                    abi::active_pool::events::ActivePoolLusdDebtUpdated::match_and_decode(log)
                {
                    events
                        .active_pool_lusd_debt_updated
                        .push(ActivePoolLusdDebtUpdated {
                            id: id.clone(),
                            lusd_debt: ev.lusd_debt.to_string(),
                            tx_hash: tx_hash.clone(),
                            log_index: log.index as u64,
                            block_num: block.number,
                            timestamp,
                        });
                }
                continue;
            }

            if log.address == COLL_SURPLUS_POOL {
                if let Some(ev) =
                    abi::coll_surplus_pool::events::CollBalanceUpdated::match_and_decode(log)
                {
                    events.coll_balance_updated.push(CollBalanceUpdated {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        new_balance: ev.new_balance.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if log.address == STABILITY_POOL {
                if let Some(ev) =
                    abi::stability_pool::events::EthGainWithdrawn::match_and_decode(log)
                {
                    events.eth_gain_withdrawn.push(EthGainWithdrawn {
                        id: id.clone(),
                        depositor: fmt_addr(&ev.depositor),
                        eth: ev.eth.to_string(),
                        lusd_loss: ev.lusd_loss.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::stability_pool::events::StabilityPoolEthBalanceUpdated::match_and_decode(
                        log,
                    )
                {
                    events
                        .stability_pool_eth_balance_updated
                        .push(StabilityPoolEthBalanceUpdated {
                            id: id.clone(),
                            new_balance: ev.new_balance.to_string(),
                            tx_hash: tx_hash.clone(),
                            log_index: log.index as u64,
                            block_num: block.number,
                            timestamp,
                        });
                    continue;
                }
                if let Some(ev) =
                    abi::stability_pool::events::StabilityPoolLusdBalanceUpdated::match_and_decode(
                        log,
                    )
                {
                    events
                        .stability_pool_lusd_balance_updated
                        .push(StabilityPoolLusdBalanceUpdated {
                            id: id.clone(),
                            new_balance: ev.new_balance.to_string(),
                            tx_hash: tx_hash.clone(),
                            log_index: log.index as u64,
                            block_num: block.number,
                            timestamp,
                        });
                    continue;
                }
                if let Some(ev) =
                    abi::stability_pool::events::UserDepositChanged::match_and_decode(log)
                {
                    events.user_deposit_changed.push(UserDepositChanged {
                        id: id.clone(),
                        depositor: fmt_addr(&ev.depositor),
                        new_deposit: ev.new_deposit.to_string(),
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

    for e in events.trove_updated {
        tables
            .create_row("trove_updated", &e.id)
            .set("borrower", &e.borrower)
            .set("debt", &e.debt)
            .set("coll", &e.coll)
            .set("stake", &e.stake)
            .set("operation", &e.operation)
            .set("source", &e.source)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.trove_liquidated {
        tables
            .create_row("trove_liquidated", &e.id)
            .set("borrower", &e.borrower)
            .set("debt", &e.debt)
            .set("coll", &e.coll)
            .set("operation", &e.operation)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.liquidations {
        tables
            .create_row("liquidations", &e.id)
            .set("liquidated_debt", &e.liquidated_debt)
            .set("liquidated_coll", &e.liquidated_coll)
            .set("coll_gas_compensation", &e.coll_gas_compensation)
            .set("lusd_gas_compensation", &e.lusd_gas_compensation)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.redemptions {
        tables
            .create_row("redemptions", &e.id)
            .set("attempted_lusd_amount", &e.attempted_lusd_amount)
            .set("actual_lusd_amount", &e.actual_lusd_amount)
            .set("eth_sent", &e.eth_sent)
            .set("eth_fee", &e.eth_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lusd_borrowing_fee_paid {
        tables
            .create_row("lusd_borrowing_fee_paid", &e.id)
            .set("borrower", &e.borrower)
            .set("lusd_fee", &e.lusd_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.last_good_price_updated {
        tables
            .create_row("last_good_price_updated", &e.id)
            .set("price", &e.price)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.active_pool_eth_balance_updated {
        tables
            .create_row("active_pool_eth_balance_updated", &e.id)
            .set("eth", &e.eth)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.active_pool_lusd_debt_updated {
        tables
            .create_row("active_pool_lusd_debt_updated", &e.id)
            .set("lusd_debt", &e.lusd_debt)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.coll_balance_updated {
        tables
            .create_row("coll_balance_updated", &e.id)
            .set("account", &e.account)
            .set("new_balance", &e.new_balance)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stability_pool_eth_balance_updated {
        tables
            .create_row("stability_pool_eth_balance_updated", &e.id)
            .set("new_balance", &e.new_balance)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stability_pool_lusd_balance_updated {
        tables
            .create_row("stability_pool_lusd_balance_updated", &e.id)
            .set("new_balance", &e.new_balance)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.eth_gain_withdrawn {
        tables
            .create_row("eth_gain_withdrawn", &e.id)
            .set("depositor", &e.depositor)
            .set("eth", &e.eth)
            .set("lusd_loss", &e.lusd_loss)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.user_deposit_changed {
        tables
            .create_row("user_deposit_changed", &e.id)
            .set("depositor", &e.depositor)
            .set("new_deposit", &e.new_deposit)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
