mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::friend_tech::types::v1::{
    Events, SharesTrade,
};

const SHARES: [u8; 20] = hex_literal::hex!("cf205808ed36593aa40a44f10c7f7c2f67d4a4d4");

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

            if log.address() == SHARES.as_slice() {
                if let Some(ev) =
                    abi::shares::events::Trade::match_and_decode(log)
                {
                    events.shares_trades.push(SharesTrade {
                        id: id.clone(),
                        trader: fmt_addr(&ev.trader),
                        subject: fmt_addr(&ev.subject),
                        is_buy: ev.is_buy.to_string(),
                        share_amount: ev.share_amount.to_string(),
                        eth_amount: ev.eth_amount.to_string(),
                        protocol_eth_amount: ev.protocol_eth_amount.to_string(),
                        subject_eth_amount: ev.subject_eth_amount.to_string(),
                        supply: ev.supply.to_string(),
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

    for e in events.shares_trades {
        tables
            .create_row("shares_trade", &e.id)
            .set("trader", &e.trader)
            .set("subject", &e.subject)
            .set("is_buy", &e.is_buy)
            .set("share_amount", &e.share_amount)
            .set("eth_amount", &e.eth_amount)
            .set("protocol_eth_amount", &e.protocol_eth_amount)
            .set("subject_eth_amount", &e.subject_eth_amount)
            .set("supply", &e.supply)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
