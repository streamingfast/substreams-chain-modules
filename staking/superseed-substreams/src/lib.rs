mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::superseed::types::v1::{
    Events, SuperSaleDepositTokensPurchase,
};

const SUPER_SALE_DEPOSIT: [u8; 20] = hex_literal::hex!("cfd9cb8f15a9732bc449b05d97c29244de2259b2");

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

            if log.address() == SUPER_SALE_DEPOSIT.as_slice() {
                if let Some(ev) =
                    abi::super_sale_deposit::events::TokensPurchase::match_and_decode(log)
                {
                    events.super_sale_deposit_tokens_purchases.push(SuperSaleDepositTokensPurchase {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        deposited_amount: ev.deposited_amount.to_string(),
                        purchased_tokens: ev.purchased_tokens.to_string(),
                        total_funds_collected: ev.total_funds_collected.to_string(),
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

    for e in events.super_sale_deposit_tokens_purchases {
        tables
            .create_row("super_sale_deposit_tokens_purchase", &e.id)
            .set("user", &e.user)
            .set("deposited_amount", &e.deposited_amount)
            .set("purchased_tokens", &e.purchased_tokens)
            .set("total_funds_collected", &e.total_funds_collected)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
