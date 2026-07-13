mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::origin_ether::types::v1::{
    Events, VaultAssetAllocated, VaultRedeem, VaultWithdrawalClaimed, VaultWithdrawalRequested, VaultYieldDistribution,
};

const VAULT: [u8; 20] = hex_literal::hex!("39254033945aa2e4809cc2977e7087bee48bd7ab");

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

            if log.address == VAULT {
                if let Some(ev) =
                    abi::vault::events::AssetAllocated::match_and_decode(log)
                {
                    events.vault_asset_allocateds.push(VaultAssetAllocated {
                        id: id.clone(),
                        asset: fmt_addr(&ev.asset),
                        strategy: fmt_addr(&ev.strategy),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::vault::events::Redeem::match_and_decode(log)
                {
                    events.vault_redeems.push(VaultRedeem {
                        id: id.clone(),
                        addr: fmt_addr(&ev.addr),
                        value: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::vault::events::WithdrawalRequested::match_and_decode(log)
                {
                    events.vault_withdrawal_requesteds.push(VaultWithdrawalRequested {
                        id: id.clone(),
                        withdrawer: fmt_addr(&ev.withdrawer),
                        request_id: ev.request_id.to_string(),
                        amount: ev.amount.to_string(),
                        queued: ev.queued.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::vault::events::WithdrawalClaimed::match_and_decode(log)
                {
                    events.vault_withdrawal_claimeds.push(VaultWithdrawalClaimed {
                        id: id.clone(),
                        withdrawer: fmt_addr(&ev.withdrawer),
                        request_id: ev.request_id.to_string(),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::vault::events::YieldDistribution::match_and_decode(log)
                {
                    events.vault_yield_distributions.push(VaultYieldDistribution {
                        id: id.clone(),
                        to: fmt_addr(&ev.to),
                        yield_amount: ev.yield_amount.to_string(),
                        fee: ev.fee.to_string(),
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

    for e in events.vault_asset_allocateds {
        tables
            .create_row("vault_asset_allocated", &e.id)
            .set("asset", &e.asset)
            .set("strategy", &e.strategy)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.vault_redeems {
        tables
            .create_row("vault_redeem", &e.id)
            .set("addr", &e.addr)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.vault_withdrawal_requesteds {
        tables
            .create_row("vault_withdrawal_requested", &e.id)
            .set("withdrawer", &e.withdrawer)
            .set("request_id", &e.request_id)
            .set("amount", &e.amount)
            .set("queued", &e.queued)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.vault_withdrawal_claimeds {
        tables
            .create_row("vault_withdrawal_claimed", &e.id)
            .set("withdrawer", &e.withdrawer)
            .set("request_id", &e.request_id)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.vault_yield_distributions {
        tables
            .create_row("vault_yield_distribution", &e.id)
            .set("to", &e.to)
            .set("yield_amount", &e.yield_amount)
            .set("fee", &e.fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
