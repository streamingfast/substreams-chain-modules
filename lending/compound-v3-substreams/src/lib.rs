mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::compound_v3::types::v1::{
    AbsorbCollateral, AbsorbDebt, BuyCollateral, Events, PauseAction, Supply, SupplyCollateral,
    Transfer, TransferCollateral, Withdraw, WithdrawCollateral, WithdrawReserves,
};

// Ethereum mainnet Comet proxy contracts
const KNOWN_COMETS: &[[u8; 20]] = &[
    hex_literal::hex!("c3d688B66703497DAA19211EEdff47f25384cdc3"), // cUSDCv3
    hex_literal::hex!("A17581A9E3356d9A858b789D68B4d866e593aE94"), // cWETHv3
    hex_literal::hex!("3Afdc9BCA9213A35503b077a6072F3D0d5AB0840"), // cUSDTv3
];

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

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for (log, _call) in trx.logs_with_calls() {
            if !KNOWN_COMETS.contains(&log.address.as_slice().try_into().unwrap_or([0u8; 20])) {
                continue;
            }

            let comet = fmt_addr(&log.address);
            let id = format!("{}-{}", tx_hash, log.index);

            if let Some(ev) = abi::comet::events::Supply::match_and_decode(log) {
                events.supplies.push(Supply {
                    id,
                    comet,
                    from: fmt_addr(&ev.from),
                    dst: fmt_addr(&ev.dst),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::SupplyCollateral::match_and_decode(log) {
                events.supply_collaterals.push(SupplyCollateral {
                    id,
                    comet,
                    from: fmt_addr(&ev.from),
                    dst: fmt_addr(&ev.dst),
                    asset: fmt_addr(&ev.asset),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::Withdraw::match_and_decode(log) {
                events.withdraws.push(Withdraw {
                    id,
                    comet,
                    src: fmt_addr(&ev.src),
                    to: fmt_addr(&ev.to),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::WithdrawCollateral::match_and_decode(log) {
                events.withdraw_collaterals.push(WithdrawCollateral {
                    id,
                    comet,
                    src: fmt_addr(&ev.src),
                    to: fmt_addr(&ev.to),
                    asset: fmt_addr(&ev.asset),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::Transfer::match_and_decode(log) {
                events.transfers.push(Transfer {
                    id,
                    comet,
                    from: fmt_addr(&ev.from),
                    to: fmt_addr(&ev.to),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::TransferCollateral::match_and_decode(log) {
                events.transfer_collaterals.push(TransferCollateral {
                    id,
                    comet,
                    from: fmt_addr(&ev.from),
                    to: fmt_addr(&ev.to),
                    asset: fmt_addr(&ev.asset),
                    amount: ev.amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::AbsorbCollateral::match_and_decode(log) {
                events.absorb_collaterals.push(AbsorbCollateral {
                    id,
                    comet,
                    absorber: fmt_addr(&ev.absorber),
                    borrower: fmt_addr(&ev.borrower),
                    asset: fmt_addr(&ev.asset),
                    collateral_absorbed: ev.collateral_absorbed.to_string(),
                    usd_value: ev.usd_value.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::AbsorbDebt::match_and_decode(log) {
                events.absorb_debts.push(AbsorbDebt {
                    id,
                    comet,
                    absorber: fmt_addr(&ev.absorber),
                    borrower: fmt_addr(&ev.borrower),
                    base_paid_out: ev.base_paid_out.to_string(),
                    usd_value: ev.usd_value.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::BuyCollateral::match_and_decode(log) {
                events.buy_collaterals.push(BuyCollateral {
                    id,
                    comet,
                    buyer: fmt_addr(&ev.buyer),
                    asset: fmt_addr(&ev.asset),
                    base_amount: ev.base_amount.to_string(),
                    collateral_amount: ev.collateral_amount.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::PauseAction::match_and_decode(log) {
                events.pause_actions.push(PauseAction {
                    id,
                    comet,
                    supply_paused: ev.supply_paused,
                    transfer_paused: ev.transfer_paused,
                    withdraw_paused: ev.withdraw_paused,
                    absorb_paused: ev.absorb_paused,
                    buy_paused: ev.buy_paused,
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::comet::events::WithdrawReserves::match_and_decode(log) {
                events.withdraw_reserves.push(WithdrawReserves {
                    id,
                    comet,
                    to: fmt_addr(&ev.to),
                    amount: ev.amount.to_string(),
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

    for e in events.supplies {
        tables
            .create_row("supplies", &e.id)
            .set("comet", &e.comet)
            .set("from", &e.from)
            .set("dst", &e.dst)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.supply_collaterals {
        tables
            .create_row("supply_collaterals", &e.id)
            .set("comet", &e.comet)
            .set("from", &e.from)
            .set("dst", &e.dst)
            .set("asset", &e.asset)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdraws {
        tables
            .create_row("withdraws", &e.id)
            .set("comet", &e.comet)
            .set("src", &e.src)
            .set("to", &e.to)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdraw_collaterals {
        tables
            .create_row("withdraw_collaterals", &e.id)
            .set("comet", &e.comet)
            .set("src", &e.src)
            .set("to", &e.to)
            .set("asset", &e.asset)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.transfers {
        tables
            .create_row("transfers", &e.id)
            .set("comet", &e.comet)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.transfer_collaterals {
        tables
            .create_row("transfer_collaterals", &e.id)
            .set("comet", &e.comet)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("asset", &e.asset)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.absorb_collaterals {
        tables
            .create_row("absorb_collaterals", &e.id)
            .set("comet", &e.comet)
            .set("absorber", &e.absorber)
            .set("borrower", &e.borrower)
            .set("asset", &e.asset)
            .set("collateral_absorbed", &e.collateral_absorbed)
            .set("usd_value", &e.usd_value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.absorb_debts {
        tables
            .create_row("absorb_debts", &e.id)
            .set("comet", &e.comet)
            .set("absorber", &e.absorber)
            .set("borrower", &e.borrower)
            .set("base_paid_out", &e.base_paid_out)
            .set("usd_value", &e.usd_value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.buy_collaterals {
        tables
            .create_row("buy_collaterals", &e.id)
            .set("comet", &e.comet)
            .set("buyer", &e.buyer)
            .set("asset", &e.asset)
            .set("base_amount", &e.base_amount)
            .set("collateral_amount", &e.collateral_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pause_actions {
        tables
            .create_row("pause_actions", &e.id)
            .set("comet", &e.comet)
            .set("supply_paused", e.supply_paused)
            .set("transfer_paused", e.transfer_paused)
            .set("withdraw_paused", e.withdraw_paused)
            .set("absorb_paused", e.absorb_paused)
            .set("buy_paused", e.buy_paused)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.withdraw_reserves {
        tables
            .create_row("withdraw_reserves", &e.id)
            .set("comet", &e.comet)
            .set("to", &e.to)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
