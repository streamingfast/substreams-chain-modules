mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::beethovenx_sftmx::types::v1::{
    Events, FtmStakingLogDeposited, FtmStakingLogUndelegated, FtmStakingLogVaultWithdrawn, FtmStakingLogWithdrawn,
};

const FTM_STAKING: [u8; 20] = hex_literal::hex!("b458bfc855ab504a8a327720fcef98886065529b");

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

            if log.address() == FTM_STAKING.as_slice() {
                if let Some(ev) =
                    abi::ftm_staking::events::LogDeposited::match_and_decode(log)
                {
                    events.ftm_staking_log_depositeds.push(FtmStakingLogDeposited {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        amount: ev.amount.to_string(),
                        ftmx_amount: ev.ftmx_amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ftm_staking::events::LogUndelegated::match_and_decode(log)
                {
                    events.ftm_staking_log_undelegateds.push(FtmStakingLogUndelegated {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        wr_id: ev.wr_id.to_string(),
                        amount_ft_mx: ev.amount_ft_mx.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ftm_staking::events::LogWithdrawn::match_and_decode(log)
                {
                    events.ftm_staking_log_withdrawns.push(FtmStakingLogWithdrawn {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        wr_id: ev.wr_id.to_string(),
                        total_amount: ev.total_amount.to_string(),
                        bitmask_to_skip: ev.bitmask_to_skip.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::ftm_staking::events::LogVaultWithdrawn::match_and_decode(log)
                {
                    events.ftm_staking_log_vault_withdrawns.push(FtmStakingLogVaultWithdrawn {
                        id: id.clone(),
                        vault: fmt_addr(&ev.vault),
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

    for e in events.ftm_staking_log_depositeds {
        tables
            .create_row("ftm_staking_log_deposited", &e.id)
            .set("user", &e.user)
            .set("amount", &e.amount)
            .set("ftmx_amount", &e.ftmx_amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ftm_staking_log_undelegateds {
        tables
            .create_row("ftm_staking_log_undelegated", &e.id)
            .set("user", &e.user)
            .set("wr_id", &e.wr_id)
            .set("amount_ft_mx", &e.amount_ft_mx)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ftm_staking_log_withdrawns {
        tables
            .create_row("ftm_staking_log_withdrawn", &e.id)
            .set("user", &e.user)
            .set("wr_id", &e.wr_id)
            .set("total_amount", &e.total_amount)
            .set("bitmask_to_skip", &e.bitmask_to_skip)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.ftm_staking_log_vault_withdrawns {
        tables
            .create_row("ftm_staking_log_vault_withdrawn", &e.id)
            .set("vault", &e.vault)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
