mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::geode::types::v1::{
    Events, GavaxTransferBatch, GavaxTransferSingle, PortalProposalApproved,
};

const PORTAL: [u8; 20] = hex_literal::hex!("4fe8c658f268842445ae8f95d4d6d8cfd356a8c8");
const GAVAX: [u8; 20] = hex_literal::hex!("6026a85e11bd895c934af02647e8c7b4ea2d9808");

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

            if log.address == PORTAL {
                if let Some(ev) =
                    abi::portal::events::ProposalApproved::match_and_decode(log)
                {
                    events.portal_proposal_approveds.push(PortalProposalApproved {
                        id: id.clone(),
                        evt_id: ev.id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == GAVAX {
                if let Some(ev) =
                    abi::gavax::events::TransferSingle::match_and_decode(log)
                {
                    events.gavax_transfer_singles.push(GavaxTransferSingle {
                        id: id.clone(),
                        operator: fmt_addr(&ev.operator),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        evt_id: ev.id.to_string(),
                        value: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::gavax::events::TransferBatch::match_and_decode(log)
                {
                    events.gavax_transfer_batchs.push(GavaxTransferBatch {
                        id: id.clone(),
                        operator: fmt_addr(&ev.operator),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        ids: ev.ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
                        values: ev.values.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
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

    for e in events.portal_proposal_approveds {
        tables
            .create_row("portal_proposal_approved", &e.id)
            .set("evt_id", &e.evt_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.gavax_transfer_singles {
        tables
            .create_row("gavax_transfer_single", &e.id)
            .set("operator", &e.operator)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("evt_id", &e.evt_id)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.gavax_transfer_batchs {
        tables
            .create_row("gavax_transfer_batch", &e.id)
            .set("operator", &e.operator)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("ids", &e.ids)
            .set("values", &e.values)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
