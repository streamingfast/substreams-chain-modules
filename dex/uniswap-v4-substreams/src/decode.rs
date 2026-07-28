use crate::abi;
use crate::pb::uniswap::v4 as v4;
use substreams::Hex;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;

// Base mainnet — Uniswap v4 (from official v4-subgraph networks.json)
const POOL_MANAGER: &str = "498581ff718922c3f8e6a244956af099b2652b2b";
const POSITION_MANAGER: &str = "7c5f5a4bbd8fd63184577525326123b519429bdc";

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

fn is_fixed(contract: &str, expected: &str) -> bool {
    contract == expected
}

fn bytes32_hex(b: &[u8]) -> String {
    hex::encode(b)
}

pub fn decode_block(blk: &eth::Block) -> v4::Events {
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

            // ---- PoolManager (fixed) ----
            if let Some(event) = abi::pool_manager::events::Initialize::match_and_decode(log) {
                if is_fixed(&contract, POOL_MANAGER) {
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
                if is_fixed(&contract, POOL_MANAGER) {
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
                if is_fixed(&contract, POOL_MANAGER) {
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

            // ---- PositionManager (fixed; Transfer is generic ERC-721) ----
            if let Some(event) = abi::position_manager::events::Transfer::match_and_decode(log) {
                if is_fixed(&contract, POSITION_MANAGER) {
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
                if is_fixed(&contract, POSITION_MANAGER) {
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
                if is_fixed(&contract, POSITION_MANAGER) {
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
