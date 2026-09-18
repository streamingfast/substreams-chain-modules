mod ctf_position;
mod pb;

use std::str::FromStr;
use substreams::errors::Error;
use substreams::scalar::BigInt;
use substreams::store::{
    StoreAdd, StoreAddBigInt, StoreGet, StoreGetBigInt, StoreGetProto, StoreNew, StoreSet,
    StoreSetProto,
};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;

use pb::polymarket::ctf::v1::CtfEvents;
use pb::polymarket::fills::v1::UnifiedFills;
use pb::polymarket::pnl::v1::{
    KeyedDelta, LatestPrice, MarketDeltas, MarketInfo, PositionLeg, TradeLegs, TradeRow,
    WhaleAlert,
};

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

// Fed by fills (map_trade_legs) and CTF splits/merges (map_ctf_legs) only —
// deliberately NOT by map_redemption_legs, so map_redemption_legs can read
// this store (see its doc comment) without creating a module-graph cycle.
// The redemption-closing amount lives in store_user_redemption_adj instead;
// db_out sums both for the displayed position.
#[substreams::handlers::store]
fn store_user_positions(trade_legs: TradeLegs, ctf_legs: TradeLegs, store: StoreAddBigInt) {
    for leg in trade_legs.legs.into_iter().chain(ctf_legs.legs) {
        store.add(0, &leg.key, bi(&leg.qty_delta));
    }
}

#[substreams::handlers::store]
fn store_user_redemption_adj(redemption_legs: TradeLegs, store: StoreAddBigInt) {
    for leg in redemption_legs.legs {
        store.add(0, &leg.key, bi(&leg.qty_delta));
    }
}

#[substreams::handlers::store]
fn store_user_cash_flow(
    trade_legs: TradeLegs,
    ctf_legs: TradeLegs,
    redemption_legs: TradeLegs,
    store: StoreAddBigInt,
) {
    for d in trade_legs
        .cash_flow_deltas
        .into_iter()
        .chain(ctf_legs.cash_flow_deltas)
        .chain(redemption_legs.cash_flow_deltas)
    {
        store.add(0, &d.key, bi(&d.delta));
    }
}

#[substreams::handlers::store]
fn store_user_volume(legs: TradeLegs, store: StoreAddBigInt) {
    for d in legs.volume_deltas {
        store.add(0, &d.key, bi(&d.delta));
    }
}

#[substreams::handlers::store]
fn store_market_volume(legs: TradeLegs, store: StoreAddBigInt) {
    for d in legs.market_volume_deltas {
        store.add(0, &d.key, bi(&d.delta));
    }
}

#[substreams::handlers::store]
fn store_latest_prices(legs: TradeLegs, store: StoreSetProto<LatestPrice>) {
    for (i, p) in legs.price_updates.into_iter().enumerate() {
        let key = p.token_id.clone();
        store.set(i as u64, &key, &p);
    }
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

// PayoutRedemption gives an aggregate `payout` but not a per-token amount:
// Gnosis's redeemPositions() burns the caller's *entire* balance of each
// index-set token. So the amount to close is read from store_user_positions
// (the running position fed by map_trade_legs + map_ctf_legs, i.e. NOT fed
// by this module) — reading a store this module doesn't also feed keeps the
// module graph acyclic. The closing amount reflects state through the end
// of the previous block; a redemption in the same block as the position's
// last contributing fill/split/merge is not expected to occur in practice
// (redemption requires the market to already be resolved).
#[substreams::handlers::map]
fn map_redemption_legs(ctf: CtfEvents, positions: StoreGetBigInt) -> Result<TradeLegs, Error> {
    let mut legs = Vec::new();
    let mut cash_flow: std::collections::HashMap<String, BigInt> = std::collections::HashMap::new();
    let mut trades = Vec::new();
    let mut whale_alerts = Vec::new();
    let zero_parent = [0u8; 32];

    for ev in &ctf.payout_redemption {
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
        let payout = bi(&ev.payout);
        let proceeds_shares = even_shares(&payout, ev.index_sets.len());

        for (i, index_set_str) in ev.index_sets.iter().enumerate() {
            let index_set = bi(index_set_str);
            let token_id = ctf_position::compute_position_id(&collateral, &condition_id, &index_set);
            let key = format!("{}:{}", ev.redeemer, token_id);
            let held = positions.get_last(&key).unwrap_or_else(|| BigInt::from(0));
            let proceeds = proceeds_shares[i].clone();

            if held.is_zero() {
                continue;
            }

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

#[substreams::handlers::map]
fn db_out(
    trade_legs: TradeLegs,
    ctf_legs: TradeLegs,
    redemption_legs: TradeLegs,
    markets: MarketDeltas,
    positions: StoreGetBigInt,
    redemption_adj: StoreGetBigInt,
    cash_flow: StoreGetBigInt,
    volume: StoreGetBigInt,
    market_volume: StoreGetBigInt,
    latest_prices: StoreGetProto<LatestPrice>,
) -> Result<DatabaseChanges, Error> {
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

    // Touched (user, token_id) position rows.
    let mut touched_position_keys: Vec<(String, String)> = Vec::new();
    let mut touched_users: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut touched_tokens: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for leg in all_legs {
        if let Some((user, token_id)) = leg.key.split_once(':') {
            touched_position_keys.push((user.to_string(), token_id.to_string()));
            touched_users.insert(user.to_string());
            touched_tokens.insert(token_id.to_string());
        }
    }

    for (user, token_id) in touched_position_keys {
        let key = format!("{}:{}", user, token_id);
        let qty = positions.get_last(&key).unwrap_or_else(|| BigInt::from(0))
            + redemption_adj.get_last(&key).unwrap_or_else(|| BigInt::from(0));
        let net_cash_flow = cash_flow.get_last(&key).unwrap_or_else(|| BigInt::from(0));
        let price = latest_prices
            .get_last(&token_id)
            .map(|p| p.price)
            .unwrap_or_default();
        let total_pnl = mark_to_market(&net_cash_flow, &qty, &price);

        tables
            .upsert_row("user_positions", [("user", user.as_str()), ("token_id", token_id.as_str())])
            .set("token_amount", qty.to_string())
            .set("net_cash_flow", net_cash_flow.to_string())
            .set("latest_price", &price)
            .set("total_pnl", total_pnl.to_string());
    }

    for user in &touched_users {
        let vol = volume.get_last(user).unwrap_or_else(|| BigInt::from(0));
        let cash = cash_flow
            .get_last(&format!("{}:ALL", user))
            .unwrap_or_else(|| BigInt::from(0));
        tables
            .upsert_row("user_pnl", user.as_str())
            .set("total_volume", vol.to_string())
            .set("net_cash_flow", cash.to_string());
    }

    for token_id in &touched_tokens {
        let _ = market_volume.get_last(token_id); // available for future markets-volume join
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

    Ok(tables.to_database_changes())
}

fn mark_to_market(cash_flow: &BigInt, qty: &BigInt, price: &str) -> BigInt {
    if price.is_empty() || qty.is_zero() {
        return cash_flow.clone();
    }
    // price is a "N.NNNNNN" fixed-point decimal string (6 decimals); qty and
    // cash_flow are raw 6-decimal atomic units, so qty * price_scaled / 1e6
    // keeps everything in the same atomic-unit scale.
    let cleaned = price.replace('.', "");
    let price_scaled = bi(&cleaned);
    let mark_value = qty.clone() * price_scaled / BigInt::from(1_000_000u64);
    cash_flow.clone() + mark_value
}
