mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::yearn::types::v1::{Events, NewExperimentalVault, NewVault, VaultTagged};

const REGISTRY_V1: [u8; 20] = hex_literal::hex!("e15461b18ee31b7379019dc523231c57d1cbc18c");
const REGISTRY_V2: [u8; 20] = hex_literal::hex!("50c1a2ea0a861a967d9d0ffe2ae4012c2e053804");

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

            let registry = if log.address == REGISTRY_V1 {
                "v1"
            } else if log.address == REGISTRY_V2 {
                "v2"
            } else {
                continue;
            };

            if let Some(ev) = abi::registry_v1::events::NewVault::match_and_decode(log) {
                events.new_vaults.push(NewVault {
                    id,
                    token: fmt_addr(&ev.token),
                    vault: fmt_addr(&ev.vault),
                    api_version: ev.api_version.clone(),
                    deployment_id: ev.deployment_id.to_string(),
                    registry: registry.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) =
                abi::registry_v1::events::NewExperimentalVault::match_and_decode(log)
            {
                events.new_experimental_vaults.push(NewExperimentalVault {
                    id,
                    token: fmt_addr(&ev.token),
                    deployer: fmt_addr(&ev.deployer),
                    vault: fmt_addr(&ev.vault),
                    api_version: ev.api_version.clone(),
                    registry: registry.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
                continue;
            }

            if let Some(ev) = abi::registry_v1::events::VaultTagged::match_and_decode(log) {
                events.vault_tagged.push(VaultTagged {
                    id,
                    vault: fmt_addr(&ev.vault),
                    tag: ev.tag.clone(),
                    registry: registry.to_string(),
                    tx_hash: tx_hash.clone(),
                    log_index: log.index as u64,
                    block_num: block.number,
                    timestamp,
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.new_vaults {
        tables
            .create_row("new_vaults", &e.id)
            .set("token", &e.token)
            .set("vault", &e.vault)
            .set("api_version", &e.api_version)
            .set("deployment_id", &e.deployment_id)
            .set("registry", &e.registry)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.new_experimental_vaults {
        tables
            .create_row("new_experimental_vaults", &e.id)
            .set("token", &e.token)
            .set("deployer", &e.deployer)
            .set("vault", &e.vault)
            .set("api_version", &e.api_version)
            .set("registry", &e.registry)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.vault_tagged {
        tables
            .create_row("vault_tagged", &e.id)
            .set("vault", &e.vault)
            .set("tag", &e.tag)
            .set("registry", &e.registry)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
