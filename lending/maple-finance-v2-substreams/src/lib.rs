mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::maple::types::v1::{Events, InstanceDeployed, LoanAddedToTransitionLoanManager};

const POOL_MANAGER_FACTORY: [u8; 20] =
    hex_literal::hex!("e463cd473ecc1d1a4ecf20b62624d84dd20a8339");
const MAPLE_LOAN_FACTORY: [u8; 20] =
    hex_literal::hex!("36a7350309b2eb30f3b908ab0154851b5ed81db0");
const LOAN_MANAGER_FACTORY: [u8; 20] =
    hex_literal::hex!("1551717ae4fdcb65ed028f7fb7aba39908f6a7a6");
const LIQUIDATOR_FACTORY: [u8; 20] =
    hex_literal::hex!("a2091116649b070d2a27fc5c85c9820302114c63");
const MIGRATION_HELPER: [u8; 20] =
    hex_literal::hex!("580b1a894b9fbdbf7d29ba9b492807bf539dd508");

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

            let factory_type = if log.address == POOL_MANAGER_FACTORY {
                "PoolManagerFactory"
            } else if log.address == MAPLE_LOAN_FACTORY {
                "MapleLoanFactory"
            } else if log.address == LOAN_MANAGER_FACTORY {
                "LoanManagerFactory"
            } else if log.address == LIQUIDATOR_FACTORY {
                "LiquidatorFactory"
            } else {
                ""
            };

            if !factory_type.is_empty() {
                if let Some(ev) =
                    abi::contract_factory::events::InstanceDeployed::match_and_decode(log)
                {
                    events.instances_deployed.push(InstanceDeployed {
                        id,
                        version: ev.version.to_string(),
                        instance: fmt_addr(&ev.instance),
                        factory_type: factory_type.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                }
                continue;
            }

            if log.address == MIGRATION_HELPER {
                if let Some(ev) =
                    abi::migration_helper::events::LoanAddedToTransitionLoanManager::match_and_decode(log)
                {
                    events.loans_added_to_transition.push(LoanAddedToTransitionLoanManager {
                        id,
                        loan_manager: fmt_addr(&ev.loan_manager),
                        loan: fmt_addr(&ev.loan),
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

    for e in events.instances_deployed {
        tables
            .create_row("instances_deployed", &e.id)
            .set("version", &e.version)
            .set("instance", &e.instance)
            .set("factory_type", &e.factory_type)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.loans_added_to_transition {
        tables
            .create_row("loans_added_to_transition", &e.id)
            .set("loan_manager", &e.loan_manager)
            .set("loan", &e.loan)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
