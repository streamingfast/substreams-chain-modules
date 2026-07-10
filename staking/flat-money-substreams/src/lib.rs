mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::flat_money::types::v1::{
    Events, DelayedOrderOrderExecuted, UnitDeposit, UnitWithdraw,
};

const UNIT: [u8; 20] = hex_literal::hex!("b95fb324b8a2faf8ec4f76e3df46c718402736e2");
const LIQUIDATION_MODULE: [u8; 20] = hex_literal::hex!("981a29dc987136d23df5a0f67d86f428fb40e8aa");
const DELAYED_ORDER: [u8; 20] = hex_literal::hex!("6d857e9d24a7566bb72a3fb0847a3e0e4e1c2879");

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

            if log.address == UNIT {
                if let Some(ev) =
                    abi::unit::events::Deposit::match_and_decode(log)
                {
                    events.unit_deposits.push(UnitDeposit {
                        id: id.clone(),
                        depositor: fmt_addr(&ev.depositor),
                        deposit_amount: ev.deposit_amount.to_string(),
                        minted_amount: ev.minted_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::unit::events::Withdraw::match_and_decode(log)
                {
                    events.unit_withdraws.push(UnitWithdraw {
                        id: id.clone(),
                        withdrawer: fmt_addr(&ev.withdrawer),
                        withdraw_amount: ev.withdraw_amount.to_string(),
                        burned_amount: ev.burned_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == LIQUIDATION_MODULE {
            }

            if log.address == DELAYED_ORDER {
                if let Some(ev) =
                    abi::delayed_order::events::OrderExecuted::match_and_decode(log)
                {
                    events.delayed_order_order_executeds.push(DelayedOrderOrderExecuted {
                        id: id.clone(),
                        account: fmt_addr(&ev.account),
                        order_type: ev.order_type.to_string(),
                        keeper_fee: ev.keeper_fee.to_string(),
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

    for e in events.unit_deposits {
        tables
            .create_row("unit_deposit", &e.id)
            .set("depositor", &e.depositor)
            .set("deposit_amount", &e.deposit_amount)
            .set("minted_amount", &e.minted_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.unit_withdraws {
        tables
            .create_row("unit_withdraw", &e.id)
            .set("withdrawer", &e.withdrawer)
            .set("withdraw_amount", &e.withdraw_amount)
            .set("burned_amount", &e.burned_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.delayed_order_order_executeds {
        tables
            .create_row("delayed_order_order_executed", &e.id)
            .set("account", &e.account)
            .set("order_type", &e.order_type)
            .set("keeper_fee", &e.keeper_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
