mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::eigenpie::types::v1::{
    Events, EigenConfigAddedNewSupportedAsset, EigenConfigReceiptTokenUpdated, EigenStakingAssetDeposit,
};

const EIGEN_CONFIG: [u8; 20] = hex_literal::hex!("20b70e4a1883b81429533fed944d7957121c7cab");
const EIGEN_STAKING: [u8; 20] = hex_literal::hex!("24db6717db1c75b9db6ea47164d8730b63875db7");

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

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == EIGEN_CONFIG.as_slice() {
                if let Some(ev) =
                    abi::eigen_config::events::AddedNewSupportedAsset::match_and_decode(log)
                {
                    events.eigen_config_added_new_supported_assets.push(EigenConfigAddedNewSupportedAsset {
                        id: id.clone(),
                        asset: fmt_addr(&ev.asset),
                        receipt: fmt_addr(&ev.receipt),
                        deposit_limit: ev.deposit_limit.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::eigen_config::events::ReceiptTokenUpdated::match_and_decode(log)
                {
                    events.eigen_config_receipt_token_updateds.push(EigenConfigReceiptTokenUpdated {
                        id: id.clone(),
                        asset: fmt_addr(&ev.asset),
                        receipt: fmt_addr(&ev.receipt),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == EIGEN_STAKING.as_slice() {
                if let Some(ev) =
                    abi::eigen_staking::events::AssetDeposit::match_and_decode(log)
                {
                    events.eigen_staking_asset_deposits.push(EigenStakingAssetDeposit {
                        id: id.clone(),
                        depositor: fmt_addr(&ev.depositor),
                        asset: fmt_addr(&ev.asset),
                        deposit_amount: ev.deposit_amount.to_string(),
                        referral: fmt_addr(&ev.referral),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
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

    for e in events.eigen_config_added_new_supported_assets {
        tables
            .create_row("eigen_config_added_new_supported_asset", &e.id)
            .set("asset", &e.asset)
            .set("receipt", &e.receipt)
            .set("deposit_limit", &e.deposit_limit)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.eigen_config_receipt_token_updateds {
        tables
            .create_row("eigen_config_receipt_token_updated", &e.id)
            .set("asset", &e.asset)
            .set("receipt", &e.receipt)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.eigen_staking_asset_deposits {
        tables
            .create_row("eigen_staking_asset_deposit", &e.id)
            .set("depositor", &e.depositor)
            .set("asset", &e.asset)
            .set("deposit_amount", &e.deposit_amount)
            .set("referral", &e.referral)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
