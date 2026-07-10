mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::euler_finance::types::v1::{
    Events, EulStakesStake, EulerAssetStatus, EulerBorrow, EulerDeposit, EulerGovConvertReserves, EulerGovSetPricingConfig, EulerGovSetReserveFee, EulerLiquidation, EulerMarketActivated, EulerRepay, EulerWithdraw,
};

const EULER: [u8; 20] = hex_literal::hex!("27182842e098f60e3d576794a5bffb0777e025d3");
const EUL_STAKES: [u8; 20] = hex_literal::hex!("c697bb6625d9f7adcf0fbf0cbd4dcf50d8716cd3");

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

            if log.address == EULER {
                if let Some(ev) =
                    abi::euler::events::AssetStatus::match_and_decode(log)
                {
                    events.euler_asset_statuss.push(EulerAssetStatus {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        total_balances: ev.total_balances.to_string(),
                        total_borrows: ev.total_borrows.to_string(),
                        reserve_balance: ev.reserve_balance.to_string(),
                        pool_size: ev.pool_size.to_string(),
                        interest_accumulator: ev.interest_accumulator.to_string(),
                        interest_rate: ev.interest_rate.to_string(),
                        evt_timestamp: ev.timestamp.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::Borrow::match_and_decode(log)
                {
                    events.euler_borrows.push(EulerBorrow {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        account: fmt_addr(&ev.account),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::Deposit::match_and_decode(log)
                {
                    events.euler_deposits.push(EulerDeposit {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        account: fmt_addr(&ev.account),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::GovConvertReserves::match_and_decode(log)
                {
                    events.euler_gov_convert_reservess.push(EulerGovConvertReserves {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        recipient: fmt_addr(&ev.recipient),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::GovSetReserveFee::match_and_decode(log)
                {
                    events.euler_gov_set_reserve_fees.push(EulerGovSetReserveFee {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        new_reserve_fee: ev.new_reserve_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::GovSetPricingConfig::match_and_decode(log)
                {
                    events.euler_gov_set_pricing_configs.push(EulerGovSetPricingConfig {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        new_pricing_type: ev.new_pricing_type.to_string(),
                        new_pricing_parameter: ev.new_pricing_parameter.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::Liquidation::match_and_decode(log)
                {
                    events.euler_liquidations.push(EulerLiquidation {
                        id: id.clone(),
                        liquidator: fmt_addr(&ev.liquidator),
                        violator: fmt_addr(&ev.violator),
                        underlying: fmt_addr(&ev.underlying),
                        collateral: fmt_addr(&ev.collateral),
                        repay: ev.repay.to_string(),
                        evt_yield: ev.evt_yield.to_string(),
                        health_score: ev.health_score.to_string(),
                        base_discount: ev.base_discount.to_string(),
                        discount: ev.discount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::MarketActivated::match_and_decode(log)
                {
                    events.euler_market_activateds.push(EulerMarketActivated {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        e_token: fmt_addr(&ev.e_token),
                        d_token: fmt_addr(&ev.d_token),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::Repay::match_and_decode(log)
                {
                    events.euler_repays.push(EulerRepay {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        account: fmt_addr(&ev.account),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::euler::events::Withdraw::match_and_decode(log)
                {
                    events.euler_withdraws.push(EulerWithdraw {
                        id: id.clone(),
                        underlying: fmt_addr(&ev.underlying),
                        account: fmt_addr(&ev.account),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == EUL_STAKES {
                if let Some(ev) =
                    abi::eul_stakes::events::Stake::match_and_decode(log)
                {
                    events.eul_stakes_stakes.push(EulStakesStake {
                        id: id.clone(),
                        who: fmt_addr(&ev.who),
                        underlying: fmt_addr(&ev.underlying),
                        sender: fmt_addr(&ev.sender),
                        new_amount: ev.new_amount.to_string(),
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

    for e in events.euler_asset_statuss {
        tables
            .create_row("euler_asset_status", &e.id)
            .set("underlying", &e.underlying)
            .set("total_balances", &e.total_balances)
            .set("total_borrows", &e.total_borrows)
            .set("reserve_balance", &e.reserve_balance)
            .set("pool_size", &e.pool_size)
            .set("interest_accumulator", &e.interest_accumulator)
            .set("interest_rate", &e.interest_rate)
            .set("evt_timestamp", &e.evt_timestamp)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_borrows {
        tables
            .create_row("euler_borrow", &e.id)
            .set("underlying", &e.underlying)
            .set("account", &e.account)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_deposits {
        tables
            .create_row("euler_deposit", &e.id)
            .set("underlying", &e.underlying)
            .set("account", &e.account)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_gov_convert_reservess {
        tables
            .create_row("euler_gov_convert_reserves", &e.id)
            .set("underlying", &e.underlying)
            .set("recipient", &e.recipient)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_gov_set_reserve_fees {
        tables
            .create_row("euler_gov_set_reserve_fee", &e.id)
            .set("underlying", &e.underlying)
            .set("new_reserve_fee", &e.new_reserve_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_gov_set_pricing_configs {
        tables
            .create_row("euler_gov_set_pricing_config", &e.id)
            .set("underlying", &e.underlying)
            .set("new_pricing_type", &e.new_pricing_type)
            .set("new_pricing_parameter", &e.new_pricing_parameter)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_liquidations {
        tables
            .create_row("euler_liquidation", &e.id)
            .set("liquidator", &e.liquidator)
            .set("violator", &e.violator)
            .set("underlying", &e.underlying)
            .set("collateral", &e.collateral)
            .set("repay", &e.repay)
            .set("evt_yield", &e.evt_yield)
            .set("health_score", &e.health_score)
            .set("base_discount", &e.base_discount)
            .set("discount", &e.discount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_market_activateds {
        tables
            .create_row("euler_market_activated", &e.id)
            .set("underlying", &e.underlying)
            .set("e_token", &e.e_token)
            .set("d_token", &e.d_token)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_repays {
        tables
            .create_row("euler_repay", &e.id)
            .set("underlying", &e.underlying)
            .set("account", &e.account)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.euler_withdraws {
        tables
            .create_row("euler_withdraw", &e.id)
            .set("underlying", &e.underlying)
            .set("account", &e.account)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.eul_stakes_stakes {
        tables
            .create_row("eul_stakes_stake", &e.id)
            .set("who", &e.who)
            .set("underlying", &e.underlying)
            .set("sender", &e.sender)
            .set("new_amount", &e.new_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
