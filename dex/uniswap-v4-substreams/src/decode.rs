use crate::abi;
use crate::pb::uniswap::v4 as v4;
use substreams::errors::Error;
use substreams::Hex;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;

/// Per-network Uniswap v4 contract addresses, supplied as module params in the
/// form `pool_manager=0x...&position_manager=0x...`.
pub struct Config {
    pool_manager: String,
    position_manager: String,
}

impl Config {
    pub fn parse(params: &str) -> Result<Self, Error> {
        let mut pool_manager = None;
        let mut position_manager = None;

        for entry in params.split('&').filter(|e| !e.trim().is_empty()) {
            let (key, value) = entry.split_once('=').ok_or_else(|| {
                Error::msg(format!("invalid params entry {entry:?}, expected key=value"))
            })?;
            match key.trim() {
                "pool_manager" => pool_manager = Some(normalize_address(key, value)?),
                "position_manager" => position_manager = Some(normalize_address(key, value)?),
                other => return Err(Error::msg(format!("unknown params key {other:?}"))),
            }
        }

        Ok(Self {
            pool_manager: pool_manager
                .ok_or_else(|| Error::msg("missing required param 'pool_manager'"))?,
            position_manager: position_manager
                .ok_or_else(|| Error::msg("missing required param 'position_manager'"))?,
        })
    }
}

/// Lowercase, un-prefixed hex, matching the encoding of `log.address`.
fn normalize_address(key: &str, value: &str) -> Result<String, Error> {
    let addr = value.trim().trim_start_matches("0x").trim_start_matches("0X").to_lowercase();
    if addr.len() != 40 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::msg(format!(
            "invalid address {value:?} for param {key:?}, expected a 20-byte hex address"
        )));
    }
    Ok(addr)
}

struct BlockMeta {
    number: u64,
    timestamp: u64,
}

fn event_id(tx: &str, log_index: u32) -> String {
    format!("{tx}-{log_index}")
}

fn addr_hex(addr: &[u8]) -> String {
    hex::encode(addr)
}

fn bytes32_hex(b: &[u8]) -> String {
    hex::encode(b)
}

pub fn decode_block(config: &Config, blk: &eth::Block) -> v4::Events {
    let mut out = v4::Events::default();
    let meta = BlockMeta {
        number: blk.number,
        timestamp: blk.timestamp_seconds(),
    };

    for receipt in blk.receipts() {
        let tx_hash = Hex(&receipt.transaction.hash).to_string();
        for log in receipt.receipt.logs.iter() {
            let contract = addr_hex(&log.address);
            let log_index = log.index;
            let id = event_id(&tx_hash, log_index);

            // ---- PoolManager (address-gated) ----
            if let Some(event) = abi::pool_manager::events::Initialize::match_and_decode(log) {
                if contract == config.pool_manager {
                    out.initialize_events.push(v4::InitializeEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Initialize".to_string(),
                        pool_id: bytes32_hex(&event.id),
                        currency0: hex::encode(&event.currency0),
                        currency1: hex::encode(&event.currency1),
                        fee: event.fee.to_string(),
                        tick_spacing: event.tick_spacing.to_string(),
                        hooks: hex::encode(&event.hooks),
                        sqrt_price_x96: event.sqrt_price_x96.to_string(),
                        tick: event.tick.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::pool_manager::events::Swap::match_and_decode(log) {
                if contract == config.pool_manager {
                    out.swap_events.push(v4::SwapEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Swap".to_string(),
                        pool_id: bytes32_hex(&event.id),
                        sender: hex::encode(&event.sender),
                        amount0: event.amount0.to_string(),
                        amount1: event.amount1.to_string(),
                        sqrt_price_x96: event.sqrt_price_x96.to_string(),
                        liquidity: event.liquidity.to_string(),
                        tick: event.tick.to_string(),
                        fee: event.fee.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::pool_manager::events::ModifyLiquidity::match_and_decode(log)
            {
                if contract == config.pool_manager {
                    out.modify_liquidity_events.push(v4::ModifyLiquidityEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "ModifyLiquidity".to_string(),
                        pool_id: bytes32_hex(&event.id),
                        sender: hex::encode(&event.sender),
                        tick_lower: event.tick_lower.to_string(),
                        tick_upper: event.tick_upper.to_string(),
                        liquidity_delta: event.liquidity_delta.to_string(),
                        salt: bytes32_hex(&event.salt),
                    });
                }
                continue;
            }

            // ---- PositionManager (address-gated; Transfer is generic ERC-721) ----
            if let Some(event) = abi::position_manager::events::Transfer::match_and_decode(log) {
                if contract == config.position_manager {
                    out.position_transfer_events.push(v4::PositionTransferEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Transfer".to_string(),
                        from_address: hex::encode(&event.from),
                        to_address: hex::encode(&event.to),
                        token_id: event.id.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) =
                abi::position_manager::events::Subscription::match_and_decode(log)
            {
                if contract == config.position_manager {
                    out.position_subscription_events
                        .push(v4::PositionSubscriptionEvent {
                            id: id.clone(),
                            block_number: meta.number,
                            block_timestamp: meta.timestamp,
                            transaction_hash: tx_hash.clone(),
                            log_index,
                            contract: contract.clone(),
                            event_name: "Subscription".to_string(),
                            token_id: event.token_id.to_string(),
                            subscriber: hex::encode(&event.subscriber),
                        });
                }
                continue;
            }

            if let Some(event) =
                abi::position_manager::events::Unsubscription::match_and_decode(log)
            {
                if contract == config.position_manager {
                    out.position_unsubscription_events
                        .push(v4::PositionUnsubscriptionEvent {
                            id: id.clone(),
                            block_number: meta.number,
                            block_timestamp: meta.timestamp,
                            transaction_hash: tx_hash.clone(),
                            log_index,
                            contract: contract.clone(),
                            event_name: "Unsubscription".to_string(),
                            token_id: event.token_id.to_string(),
                            subscriber: hex::encode(&event.subscriber),
                        });
                }
                continue;
            }
        }
    }

    out
}
