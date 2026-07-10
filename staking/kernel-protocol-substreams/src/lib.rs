mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::kernel::types::v1::{
    Events, KrethDeposit, KrethMintFeeSet, KrethRedeemFeeSet, KrethRedeemed, KsethDeposit, KsethMintFeeSet, KsethRedeemFeeSet, KsethRedeemed, KusdDeposit, KusdMintFeeSet, KusdRedeemFeeSet, KusdRedeemed,
};

const KRETH: [u8; 20] = hex_literal::hex!("f02c96dbbb92dc0325ad52b3f9f2b951f972bf00");
const KSETH: [u8; 20] = hex_literal::hex!("513d27c94c0d81eed9dc2a88b4531a69993187cf");
const KUSD: [u8; 20] = hex_literal::hex!("0bb9ab78aaf7179b7515e6753d89822b91e670c4");

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

            if log.address == KRETH {
                if let Some(ev) =
                    abi::kreth::events::MintFeeSet::match_and_decode(log)
                {
                    events.kreth_mint_fee_sets.push(KrethMintFeeSet {
                        id: id.clone(),
                        old_fee: ev.old_fee.to_string(),
                        new_fee: ev.new_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kreth::events::RedeemFeeSet::match_and_decode(log)
                {
                    events.kreth_redeem_fee_sets.push(KrethRedeemFeeSet {
                        id: id.clone(),
                        old_fee: ev.old_fee.to_string(),
                        new_fee: ev.new_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kreth::events::Deposit::match_and_decode(log)
                {
                    events.kreth_deposits.push(KrethDeposit {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        received: ev.received.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kreth::events::Redeemed::match_and_decode(log)
                {
                    events.kreth_redeemeds.push(KrethRedeemed {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        burned: ev.burned.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == KSETH {
                if let Some(ev) =
                    abi::kseth::events::MintFeeSet::match_and_decode(log)
                {
                    events.kseth_mint_fee_sets.push(KsethMintFeeSet {
                        id: id.clone(),
                        old_fee: ev.old_fee.to_string(),
                        new_fee: ev.new_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kseth::events::RedeemFeeSet::match_and_decode(log)
                {
                    events.kseth_redeem_fee_sets.push(KsethRedeemFeeSet {
                        id: id.clone(),
                        old_fee: ev.old_fee.to_string(),
                        new_fee: ev.new_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kseth::events::Deposit::match_and_decode(log)
                {
                    events.kseth_deposits.push(KsethDeposit {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        received: ev.received.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kseth::events::Redeemed::match_and_decode(log)
                {
                    events.kseth_redeemeds.push(KsethRedeemed {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        burned: ev.burned.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address == KUSD {
                if let Some(ev) =
                    abi::kusd::events::MintFeeSet::match_and_decode(log)
                {
                    events.kusd_mint_fee_sets.push(KusdMintFeeSet {
                        id: id.clone(),
                        old_fee: ev.old_fee.to_string(),
                        new_fee: ev.new_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kusd::events::RedeemFeeSet::match_and_decode(log)
                {
                    events.kusd_redeem_fee_sets.push(KusdRedeemFeeSet {
                        id: id.clone(),
                        old_fee: ev.old_fee.to_string(),
                        new_fee: ev.new_fee.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kusd::events::Deposit::match_and_decode(log)
                {
                    events.kusd_deposits.push(KusdDeposit {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        received: ev.received.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::kusd::events::Redeemed::match_and_decode(log)
                {
                    events.kusd_redeemeds.push(KusdRedeemed {
                        id: id.clone(),
                        staker: fmt_addr(&ev.staker),
                        burned: ev.burned.to_string(),
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

    for e in events.kreth_mint_fee_sets {
        tables
            .create_row("kreth_mint_fee_set", &e.id)
            .set("old_fee", &e.old_fee)
            .set("new_fee", &e.new_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kreth_redeem_fee_sets {
        tables
            .create_row("kreth_redeem_fee_set", &e.id)
            .set("old_fee", &e.old_fee)
            .set("new_fee", &e.new_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kreth_deposits {
        tables
            .create_row("kreth_deposit", &e.id)
            .set("staker", &e.staker)
            .set("received", &e.received)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kreth_redeemeds {
        tables
            .create_row("kreth_redeemed", &e.id)
            .set("staker", &e.staker)
            .set("burned", &e.burned)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kseth_mint_fee_sets {
        tables
            .create_row("kseth_mint_fee_set", &e.id)
            .set("old_fee", &e.old_fee)
            .set("new_fee", &e.new_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kseth_redeem_fee_sets {
        tables
            .create_row("kseth_redeem_fee_set", &e.id)
            .set("old_fee", &e.old_fee)
            .set("new_fee", &e.new_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kseth_deposits {
        tables
            .create_row("kseth_deposit", &e.id)
            .set("staker", &e.staker)
            .set("received", &e.received)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kseth_redeemeds {
        tables
            .create_row("kseth_redeemed", &e.id)
            .set("staker", &e.staker)
            .set("burned", &e.burned)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kusd_mint_fee_sets {
        tables
            .create_row("kusd_mint_fee_set", &e.id)
            .set("old_fee", &e.old_fee)
            .set("new_fee", &e.new_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kusd_redeem_fee_sets {
        tables
            .create_row("kusd_redeem_fee_set", &e.id)
            .set("old_fee", &e.old_fee)
            .set("new_fee", &e.new_fee)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kusd_deposits {
        tables
            .create_row("kusd_deposit", &e.id)
            .set("staker", &e.staker)
            .set("received", &e.received)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.kusd_redeemeds {
        tables
            .create_row("kusd_redeemed", &e.id)
            .set("staker", &e.staker)
            .set("burned", &e.burned)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
