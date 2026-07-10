mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::rari_vaults::types::v1::{
    Events, RariDaiFundManagerDeposit, RariDaiFundManagerWithdrawal, RariEtherFundManagerDeposit, RariEtherFundManagerWithdrawal, RariUsdcFundManagerDeposit, RariUsdcFundManagerWithdrawal, RariYieldFundManagerDeposit, RariYieldFundManagerWithdrawal,
};

const RARI_USDC_FUND_MANAGER: [u8; 20] = hex_literal::hex!("c6bf8c8a55f77686720e0a88e2fd1feef58ddf4a");
const RARI_YIELD_FUND_MANAGER: [u8; 20] = hex_literal::hex!("59fa438cd0731ebf5f4cdcaf72d4960efd13fce6");
const RARI_DAI_FUND_MANAGER: [u8; 20] = hex_literal::hex!("b465baf04c087ce3ed1c266f96ca43f4847d9635");
const RARI_ETHER_FUND_MANAGER: [u8; 20] = hex_literal::hex!("d6e194af3d9674b62d1b30ec676030c23961275e");

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

            if log.address == RARI_USDC_FUND_MANAGER {
                if let Some(ev) =
                    abi::rari_usdc_fund_manager::events::Deposit::match_and_decode(log)
                {
                    events.rari_usdc_fund_manager_deposits.push(RariUsdcFundManagerDeposit {
                        id: id.clone(),
                        currency_code: fmt_addr(&ev.currency_code.hash),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        amount_usd: ev.amount_usd.to_string(),
                        rft_minted: ev.rft_minted.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::rari_usdc_fund_manager::events::Withdrawal::match_and_decode(log)
                {
                    events.rari_usdc_fund_manager_withdrawals.push(RariUsdcFundManagerWithdrawal {
                        id: id.clone(),
                        currency_code: fmt_addr(&ev.currency_code.hash),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        amount_usd: ev.amount_usd.to_string(),
                        rft_burned: ev.rft_burned.to_string(),
                        withdrawal_fee_rate: ev.withdrawal_fee_rate.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == RARI_YIELD_FUND_MANAGER {
                if let Some(ev) =
                    abi::rari_yield_fund_manager::events::Deposit::match_and_decode(log)
                {
                    events.rari_yield_fund_manager_deposits.push(RariYieldFundManagerDeposit {
                        id: id.clone(),
                        currency_code: fmt_addr(&ev.currency_code.hash),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        amount_usd: ev.amount_usd.to_string(),
                        rft_minted: ev.rft_minted.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::rari_yield_fund_manager::events::Withdrawal::match_and_decode(log)
                {
                    events.rari_yield_fund_manager_withdrawals.push(RariYieldFundManagerWithdrawal {
                        id: id.clone(),
                        currency_code: fmt_addr(&ev.currency_code.hash),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        amount_usd: ev.amount_usd.to_string(),
                        rft_burned: ev.rft_burned.to_string(),
                        withdrawal_fee_rate: ev.withdrawal_fee_rate.to_string(),
                        amount_transferred: ev.amount_transferred.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == RARI_DAI_FUND_MANAGER {
                if let Some(ev) =
                    abi::rari_dai_fund_manager::events::Deposit::match_and_decode(log)
                {
                    events.rari_dai_fund_manager_deposits.push(RariDaiFundManagerDeposit {
                        id: id.clone(),
                        currency_code: fmt_addr(&ev.currency_code.hash),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        amount_usd: ev.amount_usd.to_string(),
                        rft_minted: ev.rft_minted.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::rari_dai_fund_manager::events::Withdrawal::match_and_decode(log)
                {
                    events.rari_dai_fund_manager_withdrawals.push(RariDaiFundManagerWithdrawal {
                        id: id.clone(),
                        currency_code: fmt_addr(&ev.currency_code.hash),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        amount_usd: ev.amount_usd.to_string(),
                        rft_burned: ev.rft_burned.to_string(),
                        withdrawal_fee_rate: ev.withdrawal_fee_rate.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == RARI_ETHER_FUND_MANAGER {
                if let Some(ev) =
                    abi::rari_ether_fund_manager::events::Deposit::match_and_decode(log)
                {
                    events.rari_ether_fund_manager_deposits.push(RariEtherFundManagerDeposit {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        rept_minted: ev.rept_minted.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::rari_ether_fund_manager::events::Withdrawal::match_and_decode(log)
                {
                    events.rari_ether_fund_manager_withdrawals.push(RariEtherFundManagerWithdrawal {
                        id: id.clone(),
                        sender: fmt_addr(&ev.sender),
                        payee: fmt_addr(&ev.payee),
                        amount: ev.amount.to_string(),
                        rept_burned: ev.rept_burned.to_string(),
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

    for e in events.rari_usdc_fund_manager_deposits {
        tables
            .create_row("rari_usdc_fund_manager_deposit", &e.id)
            .set("currency_code", &e.currency_code)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("amount_usd", &e.amount_usd)
            .set("rft_minted", &e.rft_minted)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rari_usdc_fund_manager_withdrawals {
        tables
            .create_row("rari_usdc_fund_manager_withdrawal", &e.id)
            .set("currency_code", &e.currency_code)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("amount_usd", &e.amount_usd)
            .set("rft_burned", &e.rft_burned)
            .set("withdrawal_fee_rate", &e.withdrawal_fee_rate)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rari_yield_fund_manager_deposits {
        tables
            .create_row("rari_yield_fund_manager_deposit", &e.id)
            .set("currency_code", &e.currency_code)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("amount_usd", &e.amount_usd)
            .set("rft_minted", &e.rft_minted)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rari_yield_fund_manager_withdrawals {
        tables
            .create_row("rari_yield_fund_manager_withdrawal", &e.id)
            .set("currency_code", &e.currency_code)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("amount_usd", &e.amount_usd)
            .set("rft_burned", &e.rft_burned)
            .set("withdrawal_fee_rate", &e.withdrawal_fee_rate)
            .set("amount_transferred", &e.amount_transferred)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rari_dai_fund_manager_deposits {
        tables
            .create_row("rari_dai_fund_manager_deposit", &e.id)
            .set("currency_code", &e.currency_code)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("amount_usd", &e.amount_usd)
            .set("rft_minted", &e.rft_minted)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rari_dai_fund_manager_withdrawals {
        tables
            .create_row("rari_dai_fund_manager_withdrawal", &e.id)
            .set("currency_code", &e.currency_code)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("amount_usd", &e.amount_usd)
            .set("rft_burned", &e.rft_burned)
            .set("withdrawal_fee_rate", &e.withdrawal_fee_rate)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rari_ether_fund_manager_deposits {
        tables
            .create_row("rari_ether_fund_manager_deposit", &e.id)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("rept_minted", &e.rept_minted)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.rari_ether_fund_manager_withdrawals {
        tables
            .create_row("rari_ether_fund_manager_withdrawal", &e.id)
            .set("sender", &e.sender)
            .set("payee", &e.payee)
            .set("amount", &e.amount)
            .set("rept_burned", &e.rept_burned)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
