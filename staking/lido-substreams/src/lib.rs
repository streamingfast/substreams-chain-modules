mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::lido::types::v1::{
    Deposit, EthDistribution, Events, OracleReport, Transfer, WithdrawalClaim, WithdrawalRequest,
};

// Lido stETH proxy on Ethereum mainnet — deployed block 11473216
const LIDO: [u8; 20] = hex_literal::hex!("ae7ab96520DE3A18E5e111B5EaAb095312D7fE84");
// LidoOracle — deployed block 11473216
const LIDO_ORACLE: [u8; 20] = hex_literal::hex!("442af784A788A5bd6F42A01Ebe9F287a871243fb");
// WithdrawalQueue — deployed block 17172547
const WITHDRAWAL_QUEUE: [u8; 20] = hex_literal::hex!("889edC2eDab5f40e902b864aD4d7AdE8E412F9B1");

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
            let addr = &log.address;

            if addr == &LIDO {
                if let Some(ev) = abi::lido::events::Submitted::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.deposits.push(Deposit {
                        id,
                        sender: format!("0x{}", hex::encode(ev.sender)),
                        amount_raw: ev.amount.to_string(),
                        referral: format!("0x{}", hex::encode(ev.referral)),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) = abi::lido::events::Transfer::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.transfers.push(Transfer {
                        id,
                        from: format!("0x{}", hex::encode(ev.from)),
                        to: format!("0x{}", hex::encode(ev.to)),
                        value_raw: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) = abi::lido::events::EthDistributed::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.eth_distributions.push(EthDistribution {
                        id,
                        report_timestamp: ev.report_timestamp.to_string(),
                        pre_cl_balance: ev.pre_cl_balance.to_string(),
                        post_cl_balance: ev.post_cl_balance.to_string(),
                        withdrawals_withdrawn: ev.withdrawals_withdrawn.to_string(),
                        el_rewards_withdrawn: ev.execution_layer_rewards_withdrawn.to_string(),
                        post_buffered_ether: ev.post_buffered_ether.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if addr == &LIDO_ORACLE {
                if let Some(ev) = abi::lido_oracle::events::PostTotalShares::match_and_decode(log) {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.oracle_reports.push(OracleReport {
                        id,
                        post_total_pooled_ether: ev.post_total_pooled_ether.to_string(),
                        pre_total_pooled_ether: ev.pre_total_pooled_ether.to_string(),
                        time_elapsed: ev.time_elapsed.to_string(),
                        total_shares: ev.total_shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if addr == &WITHDRAWAL_QUEUE {
                if let Some(ev) =
                    abi::withdrawal_queue::events::WithdrawalRequested::match_and_decode(log)
                {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.withdrawal_requests.push(WithdrawalRequest {
                        id,
                        request_id: ev.request_id.to_string(),
                        requestor: format!("0x{}", hex::encode(ev.requestor)),
                        owner: format!("0x{}", hex::encode(ev.owner)),
                        amount_of_st_eth: ev.amount_of_st_eth.to_string(),
                        amount_of_shares: ev.amount_of_shares.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }

                if let Some(ev) =
                    abi::withdrawal_queue::events::WithdrawalClaimed::match_and_decode(log)
                {
                    let id = format!("{}-{}", tx_hash, log.index);
                    events.withdrawal_claims.push(WithdrawalClaim {
                        id,
                        request_id: ev.request_id.to_string(),
                        owner: format!("0x{}", hex::encode(ev.owner)),
                        receiver: format!("0x{}", hex::encode(ev.receiver)),
                        amount_of_eth: ev.amount_of_eth.to_string(),
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

    for deposit in events.deposits {
        tables
            .create_row("deposits", &deposit.id)
            .set("sender", &deposit.sender)
            .set("amount_raw", &deposit.amount_raw)
            .set("referral", &deposit.referral)
            .set("tx_hash", &deposit.tx_hash)
            .set("log_index", deposit.log_index as i64)
            .set("block_num", deposit.block_num as i64)
            .set("timestamp", deposit.timestamp as i64);
    }

    for transfer in events.transfers {
        tables
            .create_row("transfers", &transfer.id)
            .set("from", &transfer.from)
            .set("to", &transfer.to)
            .set("value_raw", &transfer.value_raw)
            .set("tx_hash", &transfer.tx_hash)
            .set("log_index", transfer.log_index as i64)
            .set("block_num", transfer.block_num as i64)
            .set("timestamp", transfer.timestamp as i64);
    }

    for eth_dist in events.eth_distributions {
        tables
            .create_row("eth_distributions", &eth_dist.id)
            .set("report_timestamp", &eth_dist.report_timestamp)
            .set("pre_cl_balance", &eth_dist.pre_cl_balance)
            .set("post_cl_balance", &eth_dist.post_cl_balance)
            .set("withdrawals_withdrawn", &eth_dist.withdrawals_withdrawn)
            .set("el_rewards_withdrawn", &eth_dist.el_rewards_withdrawn)
            .set("post_buffered_ether", &eth_dist.post_buffered_ether)
            .set("tx_hash", &eth_dist.tx_hash)
            .set("log_index", eth_dist.log_index as i64)
            .set("block_num", eth_dist.block_num as i64)
            .set("timestamp", eth_dist.timestamp as i64);
    }

    for report in events.oracle_reports {
        tables
            .create_row("oracle_reports", &report.id)
            .set("post_total_pooled_ether", &report.post_total_pooled_ether)
            .set("pre_total_pooled_ether", &report.pre_total_pooled_ether)
            .set("time_elapsed", &report.time_elapsed)
            .set("total_shares", &report.total_shares)
            .set("tx_hash", &report.tx_hash)
            .set("log_index", report.log_index as i64)
            .set("block_num", report.block_num as i64)
            .set("timestamp", report.timestamp as i64);
    }

    for req in events.withdrawal_requests {
        tables
            .create_row("withdrawal_requests", &req.id)
            .set("request_id", &req.request_id)
            .set("requestor", &req.requestor)
            .set("owner", &req.owner)
            .set("amount_of_st_eth", &req.amount_of_st_eth)
            .set("amount_of_shares", &req.amount_of_shares)
            .set("tx_hash", &req.tx_hash)
            .set("log_index", req.log_index as i64)
            .set("block_num", req.block_num as i64)
            .set("timestamp", req.timestamp as i64);
    }

    for claim in events.withdrawal_claims {
        tables
            .create_row("withdrawal_claims", &claim.id)
            .set("request_id", &claim.request_id)
            .set("owner", &claim.owner)
            .set("receiver", &claim.receiver)
            .set("amount_of_eth", &claim.amount_of_eth)
            .set("tx_hash", &claim.tx_hash)
            .set("log_index", claim.log_index as i64)
            .set("block_num", claim.block_num as i64)
            .set("timestamp", claim.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
