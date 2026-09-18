mod abi;
mod pb;

use substreams::errors::Error;
use substreams::Hex;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;

use abi::ctf_exchange_v1::events as v1e;
use abi::ctf_exchange_v2::events as v2be;
use pb::polymarket::exchange::v1 as v2pb;
use pb::polymarket::fills::v1::{
    FeeCharged, NewAdmin, NewOperator, OrderCancelled, OrderFilled, OrdersMatched,
    RemovedAdmin, RemovedOperator, TransactionContext, UnifiedAdminEvents, UnifiedFeeEvents,
    UnifiedFills, UnifiedPauseEvents, UserPaused, UserUnpaused, V1Events,
};

const CTF_EXCHANGE_V1: [u8; 20] = hex_literal::hex!("4bfb41d5b3570defd03c39a9a4d8de6bd8b8982e");
const NEG_RISK_CTF_EXCHANGE_V1: [u8; 20] =
    hex_literal::hex!("c5d563a36ae78145c45a50134d48a1215220f80a");
const CTF_EXCHANGE_V2: &str = "0xe111180000d2663c0091e4f400237545b87b996b";
// Neg Risk CTF Exchange (v2). Deployed block 85,058,176 — confirmed via
// eth_getCode binary search — and confirmed actively emitting live
// OrderFilled/OrdersMatched (identical topic0 and data width to
// CTF_EXCHANGE_V2) via raw eth_getLogs, both against Polygon RPC. Named in
// the substreams-dev landing-page FAQ draft but not read by any package
// before this change — colindickson/polymarket-exchange only covers
// CTF_EXCHANGE_V2 above.
const NEG_RISK_CTF_EXCHANGE_V2: [u8; 20] =
    hex_literal::hex!("e2222d279d744050d28e00520010520000310f59");
const NEG_RISK_CTF_EXCHANGE_V2_STR: &str = "0xe2222d279d744050d28e00520010520000310f59";

fn hex0x(b: &[u8]) -> String {
    format!("0x{}", Hex::encode(b))
}

fn is_v1_exchange(addr: &[u8]) -> bool {
    addr == CTF_EXCHANGE_V1 || addr == NEG_RISK_CTF_EXCHANGE_V1
}

fn exchange_address(addr: &[u8]) -> String {
    hex0x(addr)
}

fn tx_ctx(block: &eth::Block, log_index: u64, tx_hash: &[u8]) -> Option<TransactionContext> {
    Some(TransactionContext {
        tx_hash: hex0x(tx_hash),
        log_index,
        block_number: block.number,
        timestamp: block
            .header
            .as_ref()
            .and_then(|h| h.timestamp.as_ref())
            .map(|t| t.seconds as u64)
            .unwrap_or(0),
    })
}

#[substreams::handlers::map]
fn map_v1_events(block: eth::Block) -> Result<V1Events, Error> {
    let mut fills = UnifiedFills::default();
    let mut fee_events = UnifiedFeeEvents::default();
    let mut admin_events = UnifiedAdminEvents::default();
    let mut pause_events = UnifiedPauseEvents::default();

    for trx in block.transactions() {
        let tx_hash = trx.hash.clone();

        for (log, _call) in trx.logs_with_calls() {
            if !is_v1_exchange(&log.address) {
                continue;
            }
            let addr = exchange_address(&log.address);
            let tx = tx_ctx(&block, log.index as u64, &tx_hash);

            if let Some(ev) = v1e::OrderFilled::match_and_decode(log) {
                fills.order_filled.push(OrderFilled {
                    order_hash: ev.order_hash.to_vec(),
                    maker: hex0x(&ev.maker),
                    taker: hex0x(&ev.taker),
                    maker_asset_id: ev.maker_asset_id.to_string(),
                    taker_asset_id: ev.taker_asset_id.to_string(),
                    maker_amount_filled: ev.maker_amount_filled.to_string(),
                    taker_amount_filled: ev.taker_amount_filled.to_string(),
                    fee: ev.fee.to_string(),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v1e::OrdersMatched::match_and_decode(log) {
                fills.orders_matched.push(OrdersMatched {
                    taker_order_hash: ev.taker_order_hash.to_vec(),
                    taker_order_maker: hex0x(&ev.taker_order_maker),
                    maker_asset_id: ev.maker_asset_id.to_string(),
                    taker_asset_id: ev.taker_asset_id.to_string(),
                    maker_amount_filled: ev.maker_amount_filled.to_string(),
                    taker_amount_filled: ev.taker_amount_filled.to_string(),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v1e::OrderCancelled::match_and_decode(log) {
                fills.order_cancelled.push(OrderCancelled {
                    order_hash: ev.order_hash.to_vec(),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v1e::FeeCharged::match_and_decode(log) {
                fee_events.fee_charged.push(FeeCharged {
                    receiver: hex0x(&ev.receiver),
                    token_id: ev.token_id.to_string(),
                    amount: ev.amount.to_string(),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v1e::NewAdmin::match_and_decode(log) {
                admin_events.new_admin.push(NewAdmin {
                    new_admin_address: hex0x(&ev.new_admin_address),
                    admin: hex0x(&ev.admin),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v1e::NewOperator::match_and_decode(log) {
                admin_events.new_operator.push(NewOperator {
                    new_operator_address: hex0x(&ev.new_operator_address),
                    admin: hex0x(&ev.admin),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v1e::RemovedAdmin::match_and_decode(log) {
                admin_events.removed_admin.push(RemovedAdmin {
                    removed_admin: hex0x(&ev.removed_admin),
                    admin: hex0x(&ev.admin),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v1e::RemovedOperator::match_and_decode(log) {
                admin_events.removed_operator.push(RemovedOperator {
                    removed_operator: hex0x(&ev.removed_operator),
                    admin: hex0x(&ev.admin),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(_ev) = v1e::TradingPaused::match_and_decode(log) {
                pause_events.user_paused.push(UserPaused {
                    user: String::new(),
                    effective_pause_block: String::new(),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
            if let Some(_ev) = v1e::TradingUnpaused::match_and_decode(log) {
                pause_events.user_unpaused.push(UserUnpaused {
                    user: String::new(),
                    exchange_version: 1,
                    exchange_address: addr.clone(),
                    tx,
                });
                continue;
            }
        }
    }

    Ok(V1Events {
        fills: Some(fills),
        fee_events: Some(fee_events),
        admin_events: Some(admin_events),
        pause_events: Some(pause_events),
    })
}

// Side::BUY (0): maker gives collateral (asset id "0"), taker gives the
// outcome token (token_id). Side::SELL (1): reversed. Confirmed against
// Polymarket/ctf-exchange-v2 src/exchange/libraries/Structs.sol and
// CalculatorHelper.sol.
fn v2_asset_ids(side: u32, token_id: &str) -> (String, String) {
    if side == 0 {
        ("0".to_string(), token_id.to_string())
    } else {
        (token_id.to_string(), "0".to_string())
    }
}

fn v2_tx(tx: &Option<v2pb::TransactionContext>) -> Option<TransactionContext> {
    tx.as_ref().map(|t| TransactionContext {
        tx_hash: t.tx_hash.clone(),
        log_index: t.log_index,
        block_number: t.block_number,
        timestamp: t.timestamp,
    })
}

fn log_index_of(tx: &Option<TransactionContext>) -> u64 {
    tx.as_ref().map(|t| t.log_index).unwrap_or(0)
}

// OrderFilled/OrdersMatched only — the same two events already empirically
// confirmed (topic0, indexed-param count and non-indexed data width) against
// real logs from this address. FeeCharged/admin/pause coverage was
// deliberately left out for this second address: this package's fee/admin/
// pause decode for CTF_EXCHANGE_V2 mirrors colindickson's own
// polymarket-exchange output, which is empirically validated against real
// emitted events; NEG_RISK_CTF_EXCHANGE_V2 has no such reference package to
// cross-check against, and those events are rare enough (admin actions) that
// none appeared in any sampled block range to verify indexed-ness against.
// Shipping unverified would repeat exactly the class of mistake this package
// exists to avoid.
#[substreams::handlers::map]
fn map_v2b_fills(block: eth::Block) -> Result<UnifiedFills, Error> {
    let mut out = UnifiedFills::default();

    for trx in block.transactions() {
        let tx_hash = trx.hash.clone();

        for (log, _call) in trx.logs_with_calls() {
            if log.address != NEG_RISK_CTF_EXCHANGE_V2 {
                continue;
            }
            let tx = tx_ctx(&block, log.index as u64, &tx_hash);

            if let Some(ev) = v2be::OrderFilled::match_and_decode(log) {
                let (maker_asset_id, taker_asset_id) =
                    v2_asset_ids(Into::<u32>::into(ev.side.clone()), &ev.token_id.to_string());
                out.order_filled.push(OrderFilled {
                    order_hash: ev.order_hash.to_vec(),
                    maker: hex0x(&ev.maker),
                    taker: hex0x(&ev.taker),
                    maker_asset_id,
                    taker_asset_id,
                    maker_amount_filled: ev.maker_amount_filled.to_string(),
                    taker_amount_filled: ev.taker_amount_filled.to_string(),
                    fee: ev.fee.to_string(),
                    exchange_version: 2,
                    exchange_address: NEG_RISK_CTF_EXCHANGE_V2_STR.to_string(),
                    tx,
                });
                continue;
            }
            if let Some(ev) = v2be::OrdersMatched::match_and_decode(log) {
                let (maker_asset_id, taker_asset_id) =
                    v2_asset_ids(Into::<u32>::into(ev.side.clone()), &ev.token_id.to_string());
                out.orders_matched.push(OrdersMatched {
                    taker_order_hash: ev.taker_order_hash.to_vec(),
                    taker_order_maker: hex0x(&ev.taker_order_maker),
                    maker_asset_id,
                    taker_asset_id,
                    maker_amount_filled: ev.maker_amount_filled.to_string(),
                    taker_amount_filled: ev.taker_amount_filled.to_string(),
                    exchange_version: 2,
                    exchange_address: NEG_RISK_CTF_EXCHANGE_V2_STR.to_string(),
                    tx,
                });
                continue;
            }
        }
    }

    Ok(out)
}

#[substreams::handlers::map]
fn map_fills(
    v1: V1Events,
    v2: v2pb::ExchangeEvents,
    v2b: UnifiedFills,
) -> Result<UnifiedFills, Error> {
    let mut out = v1.fills.unwrap_or_default();
    out.order_filled.extend(v2b.order_filled);
    out.orders_matched.extend(v2b.orders_matched);
    out.order_cancelled.extend(v2b.order_cancelled);

    for ev in v2.order_filled {
        let (maker_asset_id, taker_asset_id) = v2_asset_ids(ev.side, &ev.token_id);
        out.order_filled.push(OrderFilled {
            order_hash: ev.order_hash,
            maker: ev.maker,
            taker: ev.taker,
            maker_asset_id,
            taker_asset_id,
            maker_amount_filled: ev.maker_amount_filled,
            taker_amount_filled: ev.taker_amount_filled,
            fee: ev.fee,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }
    for ev in v2.orders_matched {
        let (maker_asset_id, taker_asset_id) = v2_asset_ids(ev.side, &ev.token_id);
        out.orders_matched.push(OrdersMatched {
            taker_order_hash: ev.taker_order_hash,
            taker_order_maker: ev.taker_order_maker,
            maker_asset_id,
            taker_asset_id,
            maker_amount_filled: ev.maker_amount_filled,
            taker_amount_filled: ev.taker_amount_filled,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }

    out.order_filled
        .sort_by_key(|e| log_index_of(&e.tx));
    out.orders_matched
        .sort_by_key(|e| log_index_of(&e.tx));

    Ok(out)
}

// FeeCharged only — empirically confirmed (2 topics, 32 bytes non-indexed
// data) against real logs from this address, matching CTF Exchange V2's
// FeeCharged(address indexed receiver, uint256 amount) exactly. See the
// admin/pause note on map_v2b_fills for why those events are not decoded
// here.
#[substreams::handlers::map]
fn map_v2b_fee_events(block: eth::Block) -> Result<UnifiedFeeEvents, Error> {
    let mut out = UnifiedFeeEvents::default();

    for trx in block.transactions() {
        let tx_hash = trx.hash.clone();

        for (log, _call) in trx.logs_with_calls() {
            if log.address != NEG_RISK_CTF_EXCHANGE_V2 {
                continue;
            }
            if let Some(ev) = v2be::FeeCharged::match_and_decode(log) {
                out.fee_charged.push(FeeCharged {
                    receiver: hex0x(&ev.receiver),
                    token_id: String::new(),
                    amount: ev.amount.to_string(),
                    exchange_version: 2,
                    exchange_address: NEG_RISK_CTF_EXCHANGE_V2_STR.to_string(),
                    tx: tx_ctx(&block, log.index as u64, &tx_hash),
                });
            }
        }
    }

    Ok(out)
}

#[substreams::handlers::map]
fn map_fee_events(
    v1: V1Events,
    v2: v2pb::FeeEvents,
    v2b: UnifiedFeeEvents,
) -> Result<UnifiedFeeEvents, Error> {
    let mut out = v1.fee_events.unwrap_or_default();
    out.fee_charged.extend(v2b.fee_charged);

    for ev in v2.fee_charged {
        out.fee_charged.push(FeeCharged {
            receiver: ev.recipient,
            token_id: String::new(),
            amount: ev.amount,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }
    for ev in v2.fee_receiver_updated {
        out.fee_receiver_updated
            .push(pb::polymarket::fills::v1::FeeReceiverUpdated {
                fee_receiver: ev.fee_receiver,
                exchange_version: 2,
                exchange_address: CTF_EXCHANGE_V2.to_string(),
                tx: v2_tx(&ev.tx),
            });
    }
    for ev in v2.max_fee_rate_updated {
        out.max_fee_rate_updated
            .push(pb::polymarket::fills::v1::MaxFeeRateUpdated {
                max_fee_rate: ev.max_fee_rate,
                exchange_version: 2,
                exchange_address: CTF_EXCHANGE_V2.to_string(),
                tx: v2_tx(&ev.tx),
            });
    }

    Ok(out)
}

#[substreams::handlers::map]
fn map_admin_events(v1: V1Events, v2: v2pb::AdminEvents) -> Result<UnifiedAdminEvents, Error> {
    let mut out = v1.admin_events.unwrap_or_default();

    for ev in v2.new_admin {
        out.new_admin.push(NewAdmin {
            new_admin_address: ev.new_admin_address,
            admin: ev.admin,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }
    for ev in v2.new_operator {
        out.new_operator.push(NewOperator {
            new_operator_address: ev.new_operator_address,
            admin: ev.admin,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }
    for ev in v2.removed_admin {
        out.removed_admin.push(RemovedAdmin {
            removed_admin: ev.removed_admin,
            admin: ev.admin,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }
    for ev in v2.removed_operator {
        out.removed_operator.push(RemovedOperator {
            removed_operator: ev.removed_operator,
            admin: ev.admin,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }

    Ok(out)
}

#[substreams::handlers::map]
fn map_pause_events(v1: V1Events, v2: v2pb::PauseEvents) -> Result<UnifiedPauseEvents, Error> {
    let mut out = v1.pause_events.unwrap_or_default();

    for ev in v2.user_paused {
        out.user_paused.push(UserPaused {
            user: ev.user,
            effective_pause_block: ev.effective_pause_block,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }
    for ev in v2.user_unpaused {
        out.user_unpaused.push(UserUnpaused {
            user: ev.user,
            exchange_version: 2,
            exchange_address: CTF_EXCHANGE_V2.to_string(),
            tx: v2_tx(&ev.tx),
        });
    }

    Ok(out)
}
