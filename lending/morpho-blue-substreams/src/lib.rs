mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::morpho_blue::types::v1::{
    AccrueInterest, Borrow, Events, FlashLoan, Liquidate, MarketCreated, Repay, Supply,
    SupplyCollateral, Withdraw, WithdrawCollateral,
};

// MorphoBlue singleton on Ethereum mainnet — deployed block 18883124
const MORPHO_BLUE: [u8; 20] = hex_literal::hex!("BBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb");

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
            if log.address != MORPHO_BLUE {
                continue;
            }

            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) =
                abi::morpho_blue::events::CreateMarket::match_and_decode(log)
            {
                events.markets_created.push(MarketCreated {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    loan_token: fmt_addr(&ev.market_params.0),
                    collateral_token: fmt_addr(&ev.market_params.1),
                    oracle: fmt_addr(&ev.market_params.2),
                    irm: fmt_addr(&ev.market_params.3),
                    lltv: ev.market_params.4.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::morpho_blue::events::Supply::match_and_decode(log) {
                events.supplies.push(Supply {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    caller: fmt_addr(&ev.caller),
                    on_behalf: fmt_addr(&ev.on_behalf),
                    assets: ev.assets.to_string(),
                    shares: ev.shares.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::morpho_blue::events::SupplyCollateral::match_and_decode(log)
            {
                events.supply_collaterals.push(SupplyCollateral {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    caller: fmt_addr(&ev.caller),
                    on_behalf: fmt_addr(&ev.on_behalf),
                    assets: ev.assets.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::morpho_blue::events::Borrow::match_and_decode(log) {
                events.borrows.push(Borrow {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    caller: fmt_addr(&ev.caller),
                    on_behalf: fmt_addr(&ev.on_behalf),
                    receiver: fmt_addr(&ev.receiver),
                    assets: ev.assets.to_string(),
                    shares: ev.shares.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::morpho_blue::events::Repay::match_and_decode(log) {
                events.repays.push(Repay {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    caller: fmt_addr(&ev.caller),
                    on_behalf: fmt_addr(&ev.on_behalf),
                    assets: ev.assets.to_string(),
                    shares: ev.shares.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::morpho_blue::events::Withdraw::match_and_decode(log) {
                events.withdraws.push(Withdraw {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    caller: fmt_addr(&ev.caller),
                    on_behalf: fmt_addr(&ev.on_behalf),
                    receiver: fmt_addr(&ev.receiver),
                    assets: ev.assets.to_string(),
                    shares: ev.shares.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::morpho_blue::events::WithdrawCollateral::match_and_decode(log)
            {
                events.withdraw_collaterals.push(WithdrawCollateral {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    caller: fmt_addr(&ev.caller),
                    on_behalf: fmt_addr(&ev.on_behalf),
                    receiver: fmt_addr(&ev.receiver),
                    assets: ev.assets.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::morpho_blue::events::Liquidate::match_and_decode(log) {
                events.liquidates.push(Liquidate {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    caller: fmt_addr(&ev.caller),
                    borrower: fmt_addr(&ev.borrower),
                    repaid_assets: ev.repaid_assets.to_string(),
                    repaid_shares: ev.repaid_shares.to_string(),
                    seized_assets: ev.seized_assets.to_string(),
                    bad_debt_assets: ev.bad_debt_assets.to_string(),
                    bad_debt_shares: ev.bad_debt_shares.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::morpho_blue::events::AccrueInterest::match_and_decode(log)
            {
                events.accrued_interests.push(AccrueInterest {
                    id,
                    market_id: fmt_bytes32(&ev.id),
                    prev_borrow_rate: ev.prev_borrow_rate.to_string(),
                    interest: ev.interest.to_string(),
                    fee_shares: ev.fee_shares.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::morpho_blue::events::FlashLoan::match_and_decode(log) {
                events.flash_loans.push(FlashLoan {
                    id,
                    caller: fmt_addr(&ev.caller),
                    token: fmt_addr(&ev.token),
                    assets: ev.assets.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.markets_created {
        tables
            .create_row("markets", &e.market_id)
            .set("loan_token", &e.loan_token)
            .set("collateral_token", &e.collateral_token)
            .set("oracle", &e.oracle)
            .set("irm", &e.irm)
            .set("lltv", &e.lltv)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.supplies {
        tables
            .create_row("supplies", &e.id)
            .set("market_id", &e.market_id)
            .set("caller", &e.caller)
            .set("on_behalf", &e.on_behalf)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.supply_collaterals {
        tables
            .create_row("supply_collaterals", &e.id)
            .set("market_id", &e.market_id)
            .set("caller", &e.caller)
            .set("on_behalf", &e.on_behalf)
            .set("assets", &e.assets)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.borrows {
        tables
            .create_row("borrows", &e.id)
            .set("market_id", &e.market_id)
            .set("caller", &e.caller)
            .set("on_behalf", &e.on_behalf)
            .set("receiver", &e.receiver)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.repays {
        tables
            .create_row("repays", &e.id)
            .set("market_id", &e.market_id)
            .set("caller", &e.caller)
            .set("on_behalf", &e.on_behalf)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdraws {
        tables
            .create_row("withdraws", &e.id)
            .set("market_id", &e.market_id)
            .set("caller", &e.caller)
            .set("on_behalf", &e.on_behalf)
            .set("receiver", &e.receiver)
            .set("assets", &e.assets)
            .set("shares", &e.shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdraw_collaterals {
        tables
            .create_row("withdraw_collaterals", &e.id)
            .set("market_id", &e.market_id)
            .set("caller", &e.caller)
            .set("on_behalf", &e.on_behalf)
            .set("receiver", &e.receiver)
            .set("assets", &e.assets)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.liquidates {
        tables
            .create_row("liquidations", &e.id)
            .set("market_id", &e.market_id)
            .set("caller", &e.caller)
            .set("borrower", &e.borrower)
            .set("repaid_assets", &e.repaid_assets)
            .set("repaid_shares", &e.repaid_shares)
            .set("seized_assets", &e.seized_assets)
            .set("bad_debt_assets", &e.bad_debt_assets)
            .set("bad_debt_shares", &e.bad_debt_shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.accrued_interests {
        tables
            .create_row("accrued_interests", &e.id)
            .set("market_id", &e.market_id)
            .set("prev_borrow_rate", &e.prev_borrow_rate)
            .set("interest", &e.interest)
            .set("fee_shares", &e.fee_shares)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.flash_loans {
        tables
            .create_row("flash_loans", &e.id)
            .set("caller", &e.caller)
            .set("token", &e.token)
            .set("assets", &e.assets)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
