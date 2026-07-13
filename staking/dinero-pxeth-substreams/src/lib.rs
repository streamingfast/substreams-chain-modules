mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::dinero::types::v1::{
    Events, PirexEthDeposit, PirexEthEmergencyWithdrawal, PirexEthInitiateRedemption, PirexEthRedeemWithPxEth, PirexEthValidatorDeposit, PirexFeesDistributeFees,
};

const PIREX_ETH: [u8; 20] = hex_literal::hex!("d664b74274dfeb538d9bac494f3a4760828b02b0");
const PIREX_FEES: [u8; 20] = hex_literal::hex!("177d685384aa1ac5aba41b7e649f9fa0be717fdb");

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

            if log.address == PIREX_ETH {
                if let Some(ev) =
                    abi::pirex_eth::events::Deposit::match_and_decode(log)
                {
                    events.pirex_eth_deposits.push(PirexEthDeposit {
                        id: id.clone(),
                        caller: fmt_addr(&ev.caller),
                        receiver: fmt_addr(&ev.receiver),
                        should_compound: ev.should_compound.to_string(),
                        deposited: ev.deposited.to_string(),
                        received_amount: ev.received_amount.to_string(),
                        fee_amount: ev.fee_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pirex_eth::events::ValidatorDeposit::match_and_decode(log)
                {
                    events.pirex_eth_validator_deposits.push(PirexEthValidatorDeposit {
                        id: id.clone(),
                        pub_key: fmt_addr(&ev.pub_key),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pirex_eth::events::EmergencyWithdrawal::match_and_decode(log)
                {
                    events.pirex_eth_emergency_withdrawals.push(PirexEthEmergencyWithdrawal {
                        id: id.clone(),
                        receiver: fmt_addr(&ev.receiver),
                        token: fmt_addr(&ev.token),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pirex_eth::events::InitiateRedemption::match_and_decode(log)
                {
                    events.pirex_eth_initiate_redemptions.push(PirexEthInitiateRedemption {
                        id: id.clone(),
                        assets: ev.assets.to_string(),
                        post_fee_amount: ev.post_fee_amount.to_string(),
                        receiver: fmt_addr(&ev.receiver),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::pirex_eth::events::RedeemWithPxEth::match_and_decode(log)
                {
                    events.pirex_eth_redeem_with_px_eths.push(PirexEthRedeemWithPxEth {
                        id: id.clone(),
                        assets: ev.assets.to_string(),
                        post_fee_amount: ev.post_fee_amount.to_string(),
                        receiver: fmt_addr(&ev.receiver),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == PIREX_FEES {
                if let Some(ev) =
                    abi::pirex_fees::events::DistributeFees::match_and_decode(log)
                {
                    events.pirex_fees_distribute_feess.push(PirexFeesDistributeFees {
                        id: id.clone(),
                        token: fmt_addr(&ev.token),
                        amount: ev.amount.to_string(),
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

    for e in events.pirex_eth_deposits {
        tables
            .create_row("pirex_eth_deposit", &e.id)
            .set("caller", &e.caller)
            .set("receiver", &e.receiver)
            .set("should_compound", &e.should_compound)
            .set("deposited", &e.deposited)
            .set("received_amount", &e.received_amount)
            .set("fee_amount", &e.fee_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pirex_eth_validator_deposits {
        tables
            .create_row("pirex_eth_validator_deposit", &e.id)
            .set("pub_key", &e.pub_key)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pirex_eth_emergency_withdrawals {
        tables
            .create_row("pirex_eth_emergency_withdrawal", &e.id)
            .set("receiver", &e.receiver)
            .set("token", &e.token)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pirex_eth_initiate_redemptions {
        tables
            .create_row("pirex_eth_initiate_redemption", &e.id)
            .set("assets", &e.assets)
            .set("post_fee_amount", &e.post_fee_amount)
            .set("receiver", &e.receiver)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pirex_eth_redeem_with_px_eths {
        tables
            .create_row("pirex_eth_redeem_with_px_eth", &e.id)
            .set("assets", &e.assets)
            .set("post_fee_amount", &e.post_fee_amount)
            .set("receiver", &e.receiver)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.pirex_fees_distribute_feess {
        tables
            .create_row("pirex_fees_distribute_fees", &e.id)
            .set("token", &e.token)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
