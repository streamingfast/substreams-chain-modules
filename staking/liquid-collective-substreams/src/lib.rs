mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::liquid_collective::types::v1::{
    Events, LsethPulledElFees, LsethUserDeposit, RedeemManagerClaimedRedeemRequest, RedeemManagerRequestedRedeem,
};

const LSETH: [u8; 20] = hex_literal::hex!("8c1bed5b9a0928467c9b1341da1d7bd5e10b6549");
const REDEEM_MANAGER: [u8; 20] = hex_literal::hex!("080b3a41390b357ad7e8097644d1dedf57ad3375");

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

            if log.address == LSETH {
                if let Some(ev) =
                    abi::lseth::events::PulledElFees::match_and_decode(log)
                {
                    events.lseth_pulled_el_feess.push(LsethPulledElFees {
                        id: id.clone(),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lseth::events::UserDeposit::match_and_decode(log)
                {
                    events.lseth_user_deposits.push(LsethUserDeposit {
                        id: id.clone(),
                        depositor: fmt_addr(&ev.depositor),
                        recipient: fmt_addr(&ev.recipient),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == REDEEM_MANAGER {
                if let Some(ev) =
                    abi::redeem_manager::events::RequestedRedeem::match_and_decode(log)
                {
                    events.redeem_manager_requested_redeems.push(RedeemManagerRequestedRedeem {
                        id: id.clone(),
                        owner: fmt_addr(&ev.owner),
                        height: ev.height.to_string(),
                        amount: ev.amount.to_string(),
                        max_redeemable_eth: ev.max_redeemable_eth.to_string(),
                        evt_id: ev.id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::redeem_manager::events::ClaimedRedeemRequest::match_and_decode(log)
                {
                    events.redeem_manager_claimed_redeem_requests.push(RedeemManagerClaimedRedeemRequest {
                        id: id.clone(),
                        redeem_request_id: ev.redeem_request_id.to_string(),
                        recipient: fmt_addr(&ev.recipient),
                        eth_amount: ev.eth_amount.to_string(),
                        ls_eth_amount: ev.ls_eth_amount.to_string(),
                        remaining_ls_eth_amount: ev.remaining_ls_eth_amount.to_string(),
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

    for e in events.lseth_pulled_el_feess {
        tables
            .create_row("lseth_pulled_el_fees", &e.id)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lseth_user_deposits {
        tables
            .create_row("lseth_user_deposit", &e.id)
            .set("depositor", &e.depositor)
            .set("recipient", &e.recipient)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.redeem_manager_requested_redeems {
        tables
            .create_row("redeem_manager_requested_redeem", &e.id)
            .set("owner", &e.owner)
            .set("height", &e.height)
            .set("amount", &e.amount)
            .set("max_redeemable_eth", &e.max_redeemable_eth)
            .set("evt_id", &e.evt_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.redeem_manager_claimed_redeem_requests {
        tables
            .create_row("redeem_manager_claimed_redeem_request", &e.id)
            .set("redeem_request_id", &e.redeem_request_id)
            .set("recipient", &e.recipient)
            .set("eth_amount", &e.eth_amount)
            .set("ls_eth_amount", &e.ls_eth_amount)
            .set("remaining_ls_eth_amount", &e.remaining_ls_eth_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
