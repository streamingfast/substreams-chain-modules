mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::prime_staked_eth::types::v1::{
    Events, LrtConfigAddedNewSupportedAsset, LrtDepositPoolAssetDeposit, LrtDepositPoolAssetSwapped, LrtDepositPoolWithdrawalClaimed,
};

const LRT_CONFIG: [u8; 20] = hex_literal::hex!("f879c7859b6de6fadafb74224ff05b16871646bf");
const LRT_DEPOSIT_POOL: [u8; 20] = hex_literal::hex!("a479582c8b64533102f6f528774c536e354b8d32");

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

            if log.address == LRT_CONFIG {
                if let Some(ev) =
                    abi::lrt_config::events::AddedNewSupportedAsset::match_and_decode(log)
                {
                    events.lrt_config_added_new_supported_assets.push(LrtConfigAddedNewSupportedAsset {
                        id: id.clone(),
                        asset: fmt_addr(&ev.asset),
                        deposit_limit: ev.deposit_limit.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == LRT_DEPOSIT_POOL {
                if let Some(ev) =
                    abi::lrt_deposit_pool::events::AssetDeposit::match_and_decode(log)
                {
                    events.lrt_deposit_pool_asset_deposits.push(LrtDepositPoolAssetDeposit {
                        id: id.clone(),
                        depositor: fmt_addr(&ev.depositor),
                        asset: fmt_addr(&ev.asset),
                        deposit_amount: ev.deposit_amount.to_string(),
                        prime_eth_mint_amount: ev.prime_eth_mint_amount.to_string(),
                        referral_id: ev.referral_id,
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lrt_deposit_pool::events::AssetSwapped::match_and_decode(log)
                {
                    events.lrt_deposit_pool_asset_swappeds.push(LrtDepositPoolAssetSwapped {
                        id: id.clone(),
                        from_asset: fmt_addr(&ev.from_asset),
                        to_asset: fmt_addr(&ev.to_asset),
                        from_asset_amount: ev.from_asset_amount.to_string(),
                        to_asset_amount: ev.to_asset_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::lrt_deposit_pool::events::WithdrawalClaimed::match_and_decode(log)
                {
                    events.lrt_deposit_pool_withdrawal_claimeds.push(LrtDepositPoolWithdrawalClaimed {
                        id: id.clone(),
                        withdrawer: fmt_addr(&ev.withdrawer),
                        asset: fmt_addr(&ev.asset),
                        assets: ev.assets.to_string(),
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

    for e in events.lrt_config_added_new_supported_assets {
        tables
            .create_row("lrt_config_added_new_supported_asset", &e.id)
            .set("asset", &e.asset)
            .set("deposit_limit", &e.deposit_limit)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lrt_deposit_pool_asset_deposits {
        tables
            .create_row("lrt_deposit_pool_asset_deposit", &e.id)
            .set("depositor", &e.depositor)
            .set("asset", &e.asset)
            .set("deposit_amount", &e.deposit_amount)
            .set("prime_eth_mint_amount", &e.prime_eth_mint_amount)
            .set("referral_id", &e.referral_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lrt_deposit_pool_asset_swappeds {
        tables
            .create_row("lrt_deposit_pool_asset_swapped", &e.id)
            .set("from_asset", &e.from_asset)
            .set("to_asset", &e.to_asset)
            .set("from_asset_amount", &e.from_asset_amount)
            .set("to_asset_amount", &e.to_asset_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.lrt_deposit_pool_withdrawal_claimeds {
        tables
            .create_row("lrt_deposit_pool_withdrawal_claimed", &e.id)
            .set("withdrawer", &e.withdrawer)
            .set("asset", &e.asset)
            .set("assets", &e.assets)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
