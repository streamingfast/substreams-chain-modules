mod abi;
mod decode;
mod pb;

use hex_literal::hex;
use substreams::store::{
    StoreGet, StoreGetInt64, StoreNew, StoreSetIfNotExists, StoreSetIfNotExistsInt64,
};
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;

use crate::pb::superfluid::v1 as sf;

substreams_ethereum::init!();

/// Superfluid Base mainnet fixed contracts (not data seeds).
const GOV: [u8; 20] = hex!("55f7758dd99d5e185f4cc08d4ad95b71f598264d");
const HOST: [u8; 20] = hex!("4c073b3bab6d8826b8c5b229f3cfdc1ec6e47e74");
const FACTORY: [u8; 20] = hex!("e20b9a38e0c96f61d1ba6b42a61512d56fea1eb3");
const GDA: [u8; 20] = hex!("fe6c87be05fedb2059d2ec41ba0a09826c9fd7aa");

fn addr_key(addr: &[u8]) -> String {
    hex::encode(addr)
}

fn same_addr(log_addr: &[u8], expected: &[u8; 20]) -> bool {
    log_addr.len() == 20 && log_addr == expected.as_slice()
}

/// Dynamic address discovery only (design: stores are not for HOL/aggregates).
/// SuperTokens and pools are learned **only** from on-chain factory/GDA events
/// emitted by the fixed Factory / GDA contracts (not topic-only).
///
/// Keys:
/// - `st:{addr}`   SuperToken
/// - `pool:{addr}` SuperfluidPool
/// - `gov:{addr}`  governance
#[substreams::handlers::store]
fn store_dynamic_addresses(blk: eth::Block, store: StoreSetIfNotExistsInt64) {
    store.set_if_not_exists(0, format!("gov:{}", addr_key(&GOV)), &1);

    for log in blk.logs() {
        let log = log.log;
        let emitter = log.address.as_slice();
        if same_addr(emitter, &FACTORY) {
            if let Some(ev) = abi::factory::events::SuperTokenCreated::match_and_decode(log) {
                store.set_if_not_exists(0, format!("st:{}", addr_key(&ev.token)), &1);
            }
            if let Some(ev) = abi::factory::events::CustomSuperTokenCreated::match_and_decode(log) {
                store.set_if_not_exists(0, format!("st:{}", addr_key(&ev.token)), &1);
            }
        }
        if same_addr(emitter, &GDA) {
            if let Some(ev) = abi::gda::events::PoolCreated::match_and_decode(log) {
                store.set_if_not_exists(0, format!("pool:{}", addr_key(&ev.pool)), &1);
            }
        }
        if same_addr(emitter, &HOST) {
            if let Some(ev) = abi::host::events::GovernanceReplaced::match_and_decode(log) {
                store.set_if_not_exists(0, format!("gov:{}", addr_key(&ev.new_gov)), &1);
            }
        }
    }
}

/// StoreGet-like wrapper that prefixes keys (`st:`, `pool:`, `gov:`).
pub struct PrefixedGet<'a> {
    pub inner: &'a StoreGetInt64,
    pub prefix: &'a str,
}

impl PrefixedGet<'_> {
    pub fn get_last(&self, address_hex: &str) -> Option<i64> {
        self.inner.get_last(format!("{}{}", self.prefix, address_hex))
    }
}

/// Decode Superfluid logs to typed events for ClickHouse (no HOL emission).
#[substreams::handlers::map]
fn map_events(
    blk: eth::Block,
    store: StoreGetInt64,
) -> Result<sf::Events, substreams::errors::Error> {
    let super_tokens = PrefixedGet {
        inner: &store,
        prefix: "st:",
    };
    let pools = PrefixedGet {
        inner: &store,
        prefix: "pool:",
    };
    let gov = PrefixedGet {
        inner: &store,
        prefix: "gov:",
    };

    Ok(decode::decode_block(&blk, &super_tokens, &pools, &gov))
}
