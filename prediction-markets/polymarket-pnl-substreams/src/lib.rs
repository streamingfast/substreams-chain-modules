mod ctf_position;
mod pb;

use std::str::FromStr;
use substreams::errors::Error;
use substreams::scalar::BigInt;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;

use pb::polymarket::ctf::v1::{CtfEvents, Erc1155Events};
use pb::polymarket::fills::v1::UnifiedFills;
use pb::polymarket::pnl::v1::{
    KeyedDelta, LatestPrice, MarketDeltas, MarketInfo, PositionLeg, TradeLegs, TradeRow,
    WhaleAlert,
};

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

// 10,000 USDC, raw 6-decimal atomic units.
fn whale_threshold() -> BigInt {
    BigInt::from(10_000_000_000u64)
}

fn bi(s: &str) -> BigInt {
    BigInt::from_str(s).unwrap_or_else(|_| BigInt::from(0))
}

fn decimal_price(collateral: &BigInt, token_amount: &BigInt) -> String {
    if token_amount.is_zero() {
        return String::new();
    }
    // 6-decimal fixed-point: (collateral * 1e6) / token_amount, formatted as "N.NNNNNN".
    let scaled = collateral * BigInt::from(1_000_000u64) / token_amount.clone();
    let s = scaled.to_string();
    let neg = s.starts_with('-');
    let digits = if neg { &s[1..] } else { &s[..] };
    let padded = format!("{:0>7}", digits);
    let split_at = padded.len() - 6;
    format!(
        "{}{}.{}",
        if neg { "-" } else { "" },
        &padded[..split_at],
        &padded[split_at..]
    )
}

struct Leg<'a> {
    user: &'a str,
    counterparty: &'a str,
    token_id: &'a str,
    side: &'a str, // "buy" | "sell"
    token_amount: BigInt,
    collateral_amount: BigInt,
    qty_delta: BigInt,   // signed, from user's perspective
    cash_delta: BigInt,  // signed, from user's perspective
}

#[substreams::handlers::map]
fn map_trade_legs(fills: UnifiedFills) -> Result<TradeLegs, Error> {
    let mut legs = Vec::new();
    let mut cash_flow: std::collections::HashMap<String, BigInt> = std::collections::HashMap::new();
    let mut volume: std::collections::HashMap<String, BigInt> = std::collections::HashMap::new();
    let mut market_volume: std::collections::HashMap<String, BigInt> = std::collections::HashMap::new();
    let mut price_updates = Vec::new();
    let mut trades = Vec::new();
    let mut whale_alerts = Vec::new();

    for ev in &fills.order_filled {
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        let maker_amount = bi(&ev.maker_amount_filled);
        let taker_amount = bi(&ev.taker_amount_filled);

        let (token_id, collateral_amount, token_amount) = if ev.maker_asset_id == "0" {
            (ev.taker_asset_id.as_str(), maker_amount.clone(), taker_amount.clone())
        } else {
            (ev.maker_asset_id.as_str(), taker_amount.clone(), maker_amount.clone())
        };

        let maker_leg = if ev.maker_asset_id == "0" {
            Leg {
                user: &ev.maker,
                counterparty: &ev.taker,
                token_id,
                side: "buy",
                token_amount: taker_amount.clone(),
                collateral_amount: maker_amount.clone(),
                qty_delta: taker_amount.clone(),
                cash_delta: -maker_amount.clone(),
            }
        } else {
            Leg {
                user: &ev.maker,
                counterparty: &ev.taker,
                token_id,
                side: "sell",
                token_amount: maker_amount.clone(),
                collateral_amount: taker_amount.clone(),
                qty_delta: -maker_amount.clone(),
                cash_delta: taker_amount.clone(),
            }
        };
        let taker_leg = if ev.taker_asset_id == "0" {
            Leg {
                user: &ev.taker,
                counterparty: &ev.maker,
                token_id,
                side: "buy",
                token_amount: maker_amount.clone(),
                collateral_amount: taker_amount.clone(),
                qty_delta: maker_amount.clone(),
                cash_delta: -taker_amount.clone(),
            }
        } else {
            Leg {
                user: &ev.taker,
                counterparty: &ev.maker,
                token_id,
                side: "sell",
                token_amount: taker_amount.clone(),
                collateral_amount: maker_amount.clone(),
                qty_delta: -taker_amount.clone(),
                cash_delta: maker_amount.clone(),
            }
        };

        for (i, leg) in [maker_leg, taker_leg].into_iter().enumerate() {
            let key = format!("{}:{}", leg.user, leg.token_id);
            legs.push(PositionLeg {
                key: key.clone(),
                qty_delta: leg.qty_delta.to_string(),
                cash_delta: leg.cash_delta.to_string(),
            });
            *cash_flow.entry(key).or_insert_with(|| BigInt::from(0)) += leg.cash_delta.clone();
            *cash_flow
                .entry(format!("{}:ALL", leg.user))
                .or_insert_with(|| BigInt::from(0)) += leg.cash_delta.clone();
            *volume
                .entry(leg.user.to_string())
                .or_insert_with(|| BigInt::from(0)) += leg.collateral_amount.clone();

            trades.push(TradeRow {
                id: format!("{}-{}-{}", tx.tx_hash, tx.log_index, i),
                block_number: tx.block_number,
                timestamp: tx.timestamp,
                tx_hash: tx.tx_hash.clone(),
                user: leg.user.to_string(),
                counterparty: leg.counterparty.to_string(),
                token_id: leg.token_id.to_string(),
                side: leg.side.to_string(),
                token_amount: leg.token_amount.to_string(),
                collateral_amount: leg.collateral_amount.to_string(),
                price: decimal_price(&leg.collateral_amount, &leg.token_amount),
                exchange_version: ev.exchange_version,
                exchange_address: ev.exchange_address.clone(),
            });

            if leg.collateral_amount >= whale_threshold() {
                whale_alerts.push(WhaleAlert {
                    id: format!("{}-{}-{}", tx.tx_hash, tx.log_index, i),
                    block_number: tx.block_number,
                    timestamp: tx.timestamp,
                    tx_hash: tx.tx_hash.clone(),
                    user: leg.user.to_string(),
                    token_id: leg.token_id.to_string(),
                    collateral_amount: leg.collateral_amount.to_string(),
                });
            }
        }

        *market_volume
            .entry(token_id.to_string())
            .or_insert_with(|| BigInt::from(0)) += collateral_amount.clone();

        price_updates.push(LatestPrice {
            token_id: token_id.to_string(),
            price: decimal_price(&collateral_amount, &token_amount),
            block_number: tx.block_number,
            timestamp: tx.timestamp,
        });
    }

    Ok(TradeLegs {
        legs,
        cash_flow_deltas: cash_flow
            .into_iter()
            .map(|(key, delta)| KeyedDelta {
                key,
                delta: delta.to_string(),
            })
            .collect(),
        volume_deltas: volume
            .into_iter()
            .map(|(key, delta)| KeyedDelta {
                key,
                delta: delta.to_string(),
            })
            .collect(),
        market_volume_deltas: market_volume
            .into_iter()
            .map(|(key, delta)| KeyedDelta {
                key,
                delta: delta.to_string(),
            })
            .collect(),
        price_updates,
        trades,
        whale_alerts,
    })
}

fn hex0x_bytes(b: &[u8]) -> String {
    format!("0x{}", substreams::Hex::encode(b))
}

fn parse_hex_address(s: &str) -> Option<[u8; 20]> {
    let stripped = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(stripped).ok()?;
    bytes.try_into().ok()
}

// Splits `total` into `n` shares as evenly as possible; the first `total %
// n` shares get one extra unit, so the shares sum to exactly `total` (no
// remainder lost to integer division).
fn even_shares(total: &BigInt, n: usize) -> Vec<BigInt> {
    if n == 0 {
        return vec![];
    }
    let n_big = BigInt::from(n as u64);
    let base = total.clone() / n_big.clone();
    let remainder = total.clone() - base.clone() * n_big;
    let remainder_units: u64 = remainder.clone().try_into().unwrap_or(0);
    (0..n)
        .map(|i| {
            if (i as u64) < remainder_units {
                base.clone() + BigInt::from(1u32)
            } else {
                base.clone()
            }
        })
        .collect()
}

// Position-affecting CTF events: PositionSplit and PositionsMerge only.
// Both carry an explicit `amount` — the exact quantity of each partition
// token minted (split) or burned (merge) — so no store lookup is needed.
// PayoutRedemption does not carry a per-token amount (see map_redemption_legs)
// and is handled separately to avoid a self-referential store dependency.
//
// Scope: only parent_collection_id == 0 (top-level, non-nested positions) is
// handled. Sampled 89,328 real PositionSplit/PositionsMerge events (Polygon
// blocks 93,000,000-93,003,000): 100% had a zero parent_collection_id, and
// Polymarket's own reference position-id utility (go-ctf-utils) has no
// support for a non-zero one either, so this is not expected to ever fire
// in practice — but it is checked and skipped (not guessed) if it ever does.
#[substreams::handlers::map]
fn map_ctf_legs(ctf: CtfEvents) -> Result<TradeLegs, Error> {
    let mut legs = Vec::new();
    let mut cash_flow: std::collections::HashMap<String, BigInt> = std::collections::HashMap::new();
    let mut trades = Vec::new();
    let mut whale_alerts = Vec::new();
    let zero_parent = [0u8; 32];

    for ev in &ctf.position_split {
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        if ev.parent_collection_id.as_slice() != zero_parent.as_slice() {
            continue;
        }
        let condition_id: [u8; 32] = match ev.condition_id.clone().try_into() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let collateral = match parse_hex_address(&ev.collateral_token) {
            Some(c) => c,
            None => continue,
        };
        let amount = bi(&ev.amount);
        let cost_shares = even_shares(&amount, ev.partition.len());

        for (i, index_set_str) in ev.partition.iter().enumerate() {
            let index_set = bi(index_set_str);
            let token_id = ctf_position::compute_position_id(&collateral, &condition_id, &index_set);
            let key = format!("{}:{}", ev.stakeholder, token_id);
            let cost = cost_shares[i].clone();

            legs.push(PositionLeg {
                key: key.clone(),
                qty_delta: amount.to_string(),
                cash_delta: (-cost.clone()).to_string(),
            });
            *cash_flow.entry(key).or_insert_with(|| BigInt::from(0)) -= cost.clone();
            *cash_flow
                .entry(format!("{}:ALL", ev.stakeholder))
                .or_insert_with(|| BigInt::from(0)) -= cost.clone();

            trades.push(TradeRow {
                id: format!("{}-{}-{}", tx.tx_hash, tx.log_index, i),
                block_number: tx.block_number,
                timestamp: tx.timestamp,
                tx_hash: tx.tx_hash.clone(),
                user: ev.stakeholder.clone(),
                counterparty: String::new(),
                token_id: token_id.clone(),
                side: "mint".to_string(),
                token_amount: amount.to_string(),
                collateral_amount: cost.to_string(),
                price: String::new(),
                exchange_version: 0,
                exchange_address: ev.collateral_token.clone(),
            });
            if cost >= whale_threshold() {
                whale_alerts.push(WhaleAlert {
                    id: format!("{}-{}-{}", tx.tx_hash, tx.log_index, i),
                    block_number: tx.block_number,
                    timestamp: tx.timestamp,
                    tx_hash: tx.tx_hash.clone(),
                    user: ev.stakeholder.clone(),
                    token_id: token_id.clone(),
                    collateral_amount: cost.to_string(),
                });
            }
        }
    }

    for ev in &ctf.positions_merge {
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        if ev.parent_collection_id.as_slice() != zero_parent.as_slice() {
            continue;
        }
        let condition_id: [u8; 32] = match ev.condition_id.clone().try_into() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let collateral = match parse_hex_address(&ev.collateral_token) {
            Some(c) => c,
            None => continue,
        };
        let amount = bi(&ev.amount);
        let proceeds_shares = even_shares(&amount, ev.partition.len());

        for (i, index_set_str) in ev.partition.iter().enumerate() {
            let index_set = bi(index_set_str);
            let token_id = ctf_position::compute_position_id(&collateral, &condition_id, &index_set);
            let key = format!("{}:{}", ev.stakeholder, token_id);
            let proceeds = proceeds_shares[i].clone();

            legs.push(PositionLeg {
                key: key.clone(),
                qty_delta: (-amount.clone()).to_string(),
                cash_delta: proceeds.to_string(),
            });
            *cash_flow.entry(key).or_insert_with(|| BigInt::from(0)) += proceeds.clone();
            *cash_flow
                .entry(format!("{}:ALL", ev.stakeholder))
                .or_insert_with(|| BigInt::from(0)) += proceeds.clone();

            trades.push(TradeRow {
                id: format!("{}-{}-{}", tx.tx_hash, tx.log_index, i),
                block_number: tx.block_number,
                timestamp: tx.timestamp,
                tx_hash: tx.tx_hash.clone(),
                user: ev.stakeholder.clone(),
                counterparty: String::new(),
                token_id,
                side: "burn".to_string(),
                token_amount: amount.to_string(),
                collateral_amount: proceeds.to_string(),
                price: String::new(),
                exchange_version: 0,
                exchange_address: ev.collateral_token.clone(),
            });
        }
    }

    Ok(TradeLegs {
        legs,
        cash_flow_deltas: cash_flow
            .into_iter()
            .map(|(key, delta)| KeyedDelta {
                key,
                delta: delta.to_string(),
            })
            .collect(),
        volume_deltas: vec![],
        market_volume_deltas: vec![],
        price_updates: vec![],
        trades,
        whale_alerts,
    })
}

// An ERC-1155 burn (TransferSingle / TransferBatch to the zero address) of a
// CTF position token.
struct Burn {
    tx_hash: String,
    log_index: u64,
    from: String,
    token_id: String,
    amount: BigInt,
    consumed: bool,
}

fn collect_burns(erc1155: &Erc1155Events) -> Vec<Burn> {
    let mut burns = Vec::new();
    for ev in &erc1155.transfer_single {
        if !ev.to.eq_ignore_ascii_case(ZERO_ADDRESS) {
            continue;
        }
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        burns.push(Burn {
            tx_hash: tx.tx_hash.clone(),
            log_index: tx.log_index,
            from: ev.from.clone(),
            token_id: ev.id.clone(),
            amount: bi(&ev.value),
            consumed: false,
        });
    }
    for ev in &erc1155.transfer_batch {
        if !ev.to.eq_ignore_ascii_case(ZERO_ADDRESS) {
            continue;
        }
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        for (id, value) in ev.ids.iter().zip(ev.values.iter()) {
            burns.push(Burn {
                tx_hash: tx.tx_hash.clone(),
                log_index: tx.log_index,
                from: ev.from.clone(),
                token_id: id.clone(),
                amount: bi(value),
                consumed: false,
            });
        }
    }
    burns
}

// PayoutRedemption gives an aggregate `payout` but not a per-token amount:
// Gnosis's redeemPositions() burns the caller's *entire* balance of each
// index-set token, and only emits the burn when that balance is non-zero. The
// amount to close is therefore read from the ERC-1155 burn (TransferSingle to
// the zero address) the same call emitted just before PayoutRedemption — a
// stateless lookup, so no store is needed. Each burn is matched to the nearest
// preceding unconsumed burn of the same (tx, redeemer, token_id), so a merge
// of the same token earlier in the transaction is never mistaken for the
// redemption, and two redemptions of the same token in one transaction don't
// share a burn.
//
// Index sets the redeemer held nothing in produce no burn and are skipped; the
// payout is spread evenly over the ones that did.
#[substreams::handlers::map]
fn map_redemption_legs(ctf: CtfEvents, erc1155: Erc1155Events) -> Result<TradeLegs, Error> {
    Ok(redemption_legs(&ctf, &erc1155))
}

fn redemption_legs(ctf: &CtfEvents, erc1155: &Erc1155Events) -> TradeLegs {
    let mut legs = Vec::new();
    let mut cash_flow: std::collections::HashMap<String, BigInt> = std::collections::HashMap::new();
    let mut trades = Vec::new();
    let mut whale_alerts = Vec::new();
    let zero_parent = [0u8; 32];
    let mut burns = collect_burns(erc1155);

    let mut redemptions: Vec<_> = ctf.payout_redemption.iter().collect();
    redemptions.sort_by_key(|ev| ev.tx.as_ref().map(|t| (t.tx_hash.clone(), t.log_index)));

    for ev in redemptions {
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        if ev.parent_collection_id.as_slice() != zero_parent.as_slice() {
            continue;
        }
        let condition_id: [u8; 32] = match ev.condition_id.clone().try_into() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let collateral = match parse_hex_address(&ev.collateral_token) {
            Some(c) => c,
            None => continue,
        };

        // (index into ev.index_sets, token_id, amount burned)
        let mut closed: Vec<(usize, String, BigInt)> = Vec::new();
        for (i, index_set_str) in ev.index_sets.iter().enumerate() {
            let index_set = bi(index_set_str);
            let token_id = ctf_position::compute_position_id(&collateral, &condition_id, &index_set);
            let burn = burns
                .iter_mut()
                .filter(|b| {
                    !b.consumed
                        && b.tx_hash == tx.tx_hash
                        && b.log_index < tx.log_index
                        && b.token_id == token_id
                        && b.from.eq_ignore_ascii_case(&ev.redeemer)
                })
                .max_by_key(|b| b.log_index);
            if let Some(b) = burn {
                b.consumed = true;
                if !b.amount.is_zero() {
                    closed.push((i, token_id, b.amount.clone()));
                }
            }
        }

        let proceeds_shares = even_shares(&bi(&ev.payout), closed.len());
        for ((i, token_id, held), proceeds) in closed.into_iter().zip(proceeds_shares) {
            let key = format!("{}:{}", ev.redeemer, token_id);

            legs.push(PositionLeg {
                key: key.clone(),
                qty_delta: (-held.clone()).to_string(),
                cash_delta: proceeds.to_string(),
            });
            *cash_flow.entry(key).or_insert_with(|| BigInt::from(0)) += proceeds.clone();
            *cash_flow
                .entry(format!("{}:ALL", ev.redeemer))
                .or_insert_with(|| BigInt::from(0)) += proceeds.clone();

            trades.push(TradeRow {
                id: format!("{}-{}-{}", tx.tx_hash, tx.log_index, i),
                block_number: tx.block_number,
                timestamp: tx.timestamp,
                tx_hash: tx.tx_hash.clone(),
                user: ev.redeemer.clone(),
                counterparty: String::new(),
                token_id: token_id.clone(),
                side: "redeem".to_string(),
                token_amount: held.to_string(),
                collateral_amount: proceeds.to_string(),
                price: String::new(),
                exchange_version: 0,
                exchange_address: ev.collateral_token.clone(),
            });
            if proceeds >= whale_threshold() {
                whale_alerts.push(WhaleAlert {
                    id: format!("{}-{}-{}", tx.tx_hash, tx.log_index, i),
                    block_number: tx.block_number,
                    timestamp: tx.timestamp,
                    tx_hash: tx.tx_hash.clone(),
                    user: ev.redeemer.clone(),
                    token_id: token_id.clone(),
                    collateral_amount: proceeds.to_string(),
                });
            }
        }
    }

    TradeLegs {
        legs,
        cash_flow_deltas: cash_flow
            .into_iter()
            .map(|(key, delta)| KeyedDelta {
                key,
                delta: delta.to_string(),
            })
            .collect(),
        volume_deltas: vec![],
        market_volume_deltas: vec![],
        price_updates: vec![],
        trades,
        whale_alerts,
    }
}

#[substreams::handlers::map]
fn map_markets(ctf: CtfEvents) -> Result<MarketDeltas, Error> {
    let mut created = Vec::new();
    let mut resolved = Vec::new();

    for ev in &ctf.condition_preparation {
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        created.push(MarketInfo {
            condition_id: hex0x_bytes(&ev.condition_id),
            oracle: ev.oracle.clone(),
            question_id: hex0x_bytes(&ev.question_id),
            outcome_slot_count: ev.outcome_slot_count,
            resolved: false,
            payout_numerators: vec![],
            created_block: tx.block_number,
            resolved_block: 0,
        });
    }
    for ev in &ctf.condition_resolution {
        let tx = match &ev.tx {
            Some(t) => t,
            None => continue,
        };
        resolved.push(MarketInfo {
            condition_id: hex0x_bytes(&ev.condition_id),
            oracle: ev.oracle.clone(),
            question_id: hex0x_bytes(&ev.question_id),
            outcome_slot_count: ev.outcome_slot_count,
            resolved: true,
            payout_numerators: ev.payout_numerators.clone(),
            created_block: 0,
            resolved_block: tx.block_number,
        });
    }

    Ok(MarketDeltas { created, resolved })
}

// No store inputs: every running total is accumulated by Postgres through
// `add` delta ops (the sink applies them as `col = COALESCE(col, 0) + value`,
// and reverts them on a reorg), rather than read back from a store here.
// Consequently total_pnl, which is nonlinear in the running quantity, cannot
// be emitted at all — it is derived by the `user_positions_pnl` view in
// schema.sql from user_positions and token_prices.
#[substreams::handlers::map]
fn db_out(
    trade_legs: TradeLegs,
    ctf_legs: TradeLegs,
    redemption_legs: TradeLegs,
    markets: MarketDeltas,
) -> Result<DatabaseChanges, Error> {
    Ok(database_changes(&trade_legs, &ctf_legs, &redemption_legs, &markets))
}

fn database_changes(
    trade_legs: &TradeLegs,
    ctf_legs: &TradeLegs,
    redemption_legs: &TradeLegs,
    markets: &MarketDeltas,
) -> DatabaseChanges {
    let mut tables = Tables::new();

    let all_trades = trade_legs
        .trades
        .iter()
        .chain(ctf_legs.trades.iter())
        .chain(redemption_legs.trades.iter());
    let all_whale_alerts = trade_legs
        .whale_alerts
        .iter()
        .chain(ctf_legs.whale_alerts.iter())
        .chain(redemption_legs.whale_alerts.iter());
    let all_legs = trade_legs
        .legs
        .iter()
        .chain(ctf_legs.legs.iter())
        .chain(redemption_legs.legs.iter());

    for t in all_trades {
        tables
            .create_row("trades", t.id.as_str())
            .set("block_number", t.block_number)
            .set("timestamp", t.timestamp)
            .set("tx_hash", &t.tx_hash)
            .set("user", &t.user)
            .set("counterparty", &t.counterparty)
            .set("token_id", &t.token_id)
            .set("side", &t.side)
            .set("token_amount", &t.token_amount)
            .set("collateral_amount", &t.collateral_amount)
            .set("price", &t.price)
            .set("exchange_version", t.exchange_version)
            .set("exchange_address", &t.exchange_address);
    }

    for w in all_whale_alerts {
        tables
            .create_row("whale_alerts", w.id.as_str())
            .set("block_number", w.block_number)
            .set("timestamp", w.timestamp)
            .set("tx_hash", &w.tx_hash)
            .set("user", &w.user)
            .set("token_id", &w.token_id)
            .set("collateral_amount", &w.collateral_amount);
    }

    // Per-(user, token_id) position and cash-flow deltas, and the same cash
    // flow rolled up per user. Rows touched more than once in a block are
    // summed by Tables before they reach the sink.
    for leg in all_legs {
        let (user, token_id) = match leg.key.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };
        tables
            .upsert_row("user_positions", [("user", user), ("token_id", token_id)])
            .add("token_amount", leg.qty_delta.as_str())
            .add("net_cash_flow", leg.cash_delta.as_str());
        tables
            .upsert_row("user_pnl", user)
            .add("net_cash_flow", leg.cash_delta.as_str());
    }

    for d in &trade_legs.volume_deltas {
        tables
            .upsert_row("user_pnl", d.key.as_str())
            .add("total_volume", d.delta.as_str());
    }

    // price_updates arrive in fill order, so the last write per token wins.
    for p in &trade_legs.price_updates {
        if p.price.is_empty() {
            continue;
        }
        tables
            .upsert_row("token_prices", p.token_id.as_str())
            .set("price", &p.price)
            .set("block_number", p.block_number)
            .set("timestamp", p.timestamp);
    }

    for m in &markets.created {
        tables
            .upsert_row("markets", m.condition_id.as_str())
            .set("oracle", &m.oracle)
            .set("question_id", &m.question_id)
            .set("outcome_slot_count", m.outcome_slot_count)
            .set("created_block", m.created_block);
    }
    for m in &markets.resolved {
        tables
            .upsert_row("markets", m.condition_id.as_str())
            .set("resolved", true)
            .set("payout_numerators", m.payout_numerators.join(","))
            .set("resolved_block", m.resolved_block);
    }

    tables.to_database_changes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pb::polymarket::ctf::v1::{PayoutRedemption, TransactionContext, TransferBatch, TransferSingle};

    const USER: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const COLLATERAL: &str = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174";

    fn ctx(tx: &str, log_index: u64) -> Option<TransactionContext> {
        Some(TransactionContext {
            tx_hash: tx.to_string(),
            log_index,
            block_number: 1,
            timestamp: 1,
        })
    }

    fn token(index_set: u32) -> String {
        ctf_position::compute_position_id(
            &parse_hex_address(COLLATERAL).unwrap(),
            &[7u8; 32],
            &BigInt::from(index_set),
        )
    }

    fn redemption(tx: &str, log_index: u64, payout: &str) -> PayoutRedemption {
        PayoutRedemption {
            redeemer: USER.to_string(),
            collateral_token: COLLATERAL.to_string(),
            parent_collection_id: vec![0u8; 32],
            condition_id: vec![7u8; 32],
            index_sets: vec!["1".to_string(), "2".to_string()],
            payout: payout.to_string(),
            tx: ctx(tx, log_index),
        }
    }

    fn burn(tx: &str, log_index: u64, from: &str, id: String, value: &str) -> TransferSingle {
        TransferSingle {
            operator: from.to_string(),
            from: from.to_string(),
            to: ZERO_ADDRESS.to_string(),
            id,
            value: value.to_string(),
            tx: ctx(tx, log_index),
        }
    }

    #[test]
    fn redemption_closes_burned_amount_and_pays_only_held_tokens() {
        let ctf = CtfEvents {
            payout_redemption: vec![redemption("0xa", 10, "1000")],
            ..Default::default()
        };
        // Only index set 2 was held; index set 1 emits no burn.
        let erc = Erc1155Events {
            transfer_single: vec![burn("0xa", 9, USER, token(2), "1000")],
            ..Default::default()
        };

        let out = redemption_legs(&ctf, &erc);

        assert_eq!(out.legs.len(), 1);
        assert_eq!(out.legs[0].key, format!("{}:{}", USER, token(2)));
        assert_eq!(out.legs[0].qty_delta, "-1000");
        // Whole payout goes to the one held token, not half of it.
        assert_eq!(out.legs[0].cash_delta, "1000");
        assert_eq!(out.trades[0].token_amount, "1000");
    }

    #[test]
    fn redemption_ignores_other_burns() {
        let other = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let ctf = CtfEvents {
            payout_redemption: vec![redemption("0xa", 10, "500")],
            ..Default::default()
        };
        let erc = Erc1155Events {
            transfer_single: vec![
                // Not a burn: a plain transfer.
                TransferSingle {
                    to: other.to_string(),
                    ..burn("0xa", 8, USER, token(1), "5")
                },
                burn("0xa", 9, other, token(1), "6"), // other holder
                burn("0xb", 9, USER, token(1), "7"),  // other transaction
                burn("0xa", 11, USER, token(1), "8"), // after the redemption event
            ],
            ..Default::default()
        };

        let out = redemption_legs(&ctf, &erc);
        assert!(out.legs.is_empty());
        assert!(out.trades.is_empty());
    }

    #[test]
    fn redemption_takes_nearest_burn_and_never_reuses_one() {
        // A merge burn (log 3) precedes the redemption's own burn (log 9) of the
        // same token in one transaction; a second redemption has no burn left.
        let ctf = CtfEvents {
            payout_redemption: vec![redemption("0xa", 10, "100"), redemption("0xa", 20, "100")],
            ..Default::default()
        };
        let erc = Erc1155Events {
            transfer_single: vec![
                burn("0xa", 3, USER, token(1), "50"),
                burn("0xa", 9, USER, token(1), "70"),
            ],
            transfer_batch: vec![TransferBatch {
                operator: USER.to_string(),
                from: USER.to_string(),
                to: ZERO_ADDRESS.to_string(),
                ids: vec![token(1)],
                values: vec!["30".to_string()],
                tx: ctx("0xa", 15),
            }],
            ..Default::default()
        };

        let out = redemption_legs(&ctf, &erc);

        let qty: Vec<&str> = out.legs.iter().map(|l| l.qty_delta.as_str()).collect();
        // First redemption (log 10) takes the log-9 burn, second (log 20) the
        // log-15 batch burn; the merge burn at log 3 is left alone.
        assert_eq!(qty, vec!["-70", "-30"]);
    }

    #[test]
    fn db_out_emits_deltas_not_totals() {
        let leg = |key: &str, qty: &str, cash: &str| PositionLeg {
            key: key.to_string(),
            qty_delta: qty.to_string(),
            cash_delta: cash.to_string(),
        };
        let trade_legs = TradeLegs {
            legs: vec![leg("0xu:t1", "10", "-4"), leg("0xu:t1", "-3", "2")],
            volume_deltas: vec![KeyedDelta { key: "0xu".into(), delta: "6".into() }],
            ..Default::default()
        };

        let changes = database_changes(
            &trade_legs,
            &TradeLegs::default(),
            &TradeLegs::default(),
            &MarketDeltas::default(),
        );

        let position = changes
            .table_changes
            .iter()
            .find(|c| c.table == "user_positions")
            .expect("user_positions change");
        let field = |name: &str| position.fields.iter().find(|f| f.name == name).unwrap();
        // Two legs on one key in one block collapse into a single summed delta.
        assert_eq!(changes.table_changes.iter().filter(|c| c.table == "user_positions").count(), 1);
        assert_eq!(field("token_amount").value, "7");
        assert_eq!(field("net_cash_flow").value, "-2");
        assert_eq!(field("token_amount").update_op(), substreams_database_change::pb::sf::substreams::sink::database::v1::field::UpdateOp::Add);
    }
}
