//! ClickHouse rows. Inserts only; the sink dedupes on the ReplacingMergeTree key.

use substreams::pb::substreams::Clock;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;

use crate::pb::hood::basis::v1::{BasisTicks, ChainlinkAnswers, StockSwaps};
use crate::price;
use crate::registry;

/// PoolManager deployment block; the registry snapshot is written once here.
pub const REGISTRY_BLOCK: u64 = 9070;

pub fn build(clock: &Clock, swaps: &StockSwaps, answers: &ChainlinkAnswers, ticks: &BasisTicks) -> DatabaseChanges {
    let mut tables = Tables::new();

    if clock.number == REGISTRY_BLOCK {
        for a in registry::ASSETS {
            let feed = registry::feed_by_token(a.token);
            tables
                .create_row("stock_registry", a.token)
                .set("ticker", a.ticker)
                .set("token", a.token)
                .set("feed", feed.map(|f| f.proxy).unwrap_or(""))
                .set("aggregator", feed.map(|f| f.aggregator).unwrap_or(""))
                .set("multiplier_str", dec(a.multiplier))
                .set("decimals", a.decimals)
                .set("status", a.status)
                .set("name", a.name)
                .set("has_feed", feed.is_some());
        }
    }

    for s in &swaps.swaps {
        tables
            .create_row("stock_swaps", pk(&s.tx_hash, s.log_index))
            .set("block_num", s.block_num)
            .set("block_ts", s.block_ts)
            .set("block_time", s.block_ts)
            .set("pool_id", &s.pool_id)
            .set("ticker", &s.ticker)
            .set("token", &s.token)
            .set("side", &s.side)
            .set("shares_ui_str", dec(&s.shares_ui))
            .set("shares_raw_adjusted_str", dec(&s.shares_raw_adjusted))
            .set("quote_symbol", &s.quote_symbol)
            .set("quote_kind", &s.quote_kind)
            .set("quote_amount_str", dec(&s.quote_amount))
            .set("amount_usd_str", dec(&s.amount_usd))
            .set("price_usd_str", dec(&s.price_usd))
            .set("priced", !s.price_usd.is_empty())
            .set("sender", &s.sender)
            .set("origin", &s.origin)
            .set("fee", s.fee)
            .set("hook_address", &s.hook_address);
    }

    for a in &answers.answers {
        tables
            .create_row("chainlink_answers", pk(&a.tx_hash, a.log_index))
            .set("block_num", a.block_num)
            .set("block_ts", a.block_ts)
            .set("block_time", a.block_ts)
            .set("ticker", &a.ticker)
            .set("feed", &a.feed)
            .set("aggregator", &a.aggregator)
            .set("answer_usd_str", dec(&a.answer_usd))
            .set("round_id", &a.round_id)
            .set("updated_at", a.updated_at);
    }

    for t in &ticks.ticks {
        tables
            .create_row("basis_ticks", pk(&t.tx_hash, t.log_index))
            .set("block_num", t.block_num)
            .set("block_ts", t.block_ts)
            .set("block_time", t.block_ts)
            .set("ticker", &t.ticker)
            .set("implied_usd_str", dec(&t.implied_usd))
            .set("ref_usd_str", dec(&t.ref_usd))
            .set("ref_source", &t.ref_source)
            .set("ref_ts", t.ref_ts)
            .set("premium_bps", t.premium_bps)
            .set("session", &t.session)
            .set("amount_usd_str", dec(&t.amount_usd))
            .set("side", &t.side);
    }

    tables.to_database_changes()
}

fn pk(tx_hash: &str, log_index: u32) -> [(&'static str, String); 2] {
    [("tx_hash", tx_hash.to_string()), ("log_index", log_index.to_string())]
}

// Exact decimal as a string, truncated to the Decimal(38,18) scale the
// MATERIALIZED column parses with; unpriced rows carry 0 plus a flag.
fn dec(s: &str) -> String {
    let out = price::parse(s)
        .map(|d| price::fmt(&d))
        .unwrap_or_else(|| "0".to_string());
    // Decimal(38, 18) has 20 integer digits; toDecimal128OrNull returns NULL
    // past that, so fall back to 0 rather than let it silently null out.
    if int_digits(&out) > 20 {
        return "0".to_string();
    }
    out
}

fn int_digits(s: &str) -> usize {
    let s = s.strip_prefix('-').unwrap_or(s);
    s.split('.').next().unwrap_or(s).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pb::hood::basis::v1::{BasisTick, ChainlinkAnswer, StockSwap};
    use substreams_database_change::pb::sf::substreams::sink::database::v1::table_change::Operation;

    fn clock(number: u64) -> Clock {
        Clock {
            number,
            ..Default::default()
        }
    }

    fn changes_for<'a>(
        db: &'a DatabaseChanges,
        table: &str,
    ) -> Vec<&'a substreams_database_change::pb::sf::substreams::sink::database::v1::TableChange> {
        db.table_changes.iter().filter(|c| c.table == table).collect()
    }

    #[test]
    fn registry_only_at_deployment_block() {
        let empty = (
            StockSwaps::default(),
            ChainlinkAnswers::default(),
            BasisTicks::default(),
        );
        let at = build(&clock(REGISTRY_BLOCK), &empty.0, &empty.1, &empty.2);
        let reg = changes_for(&at, "stock_registry");
        assert_eq!(reg.len(), registry::ASSETS.len());
        let with_feed = reg
            .iter()
            .filter(|c| c.fields.iter().any(|f| f.name == "has_feed" && f.value == "true"))
            .count();
        assert_eq!(with_feed, registry::FEEDS.len());

        let later = build(&clock(REGISTRY_BLOCK + 1), &empty.0, &empty.1, &empty.2);
        assert!(later.table_changes.is_empty());
    }

    #[test]
    fn rows_use_composite_keys_and_zero_for_empty_decimals() {
        let swaps = StockSwaps {
            swaps: vec![StockSwap {
                tx_hash: "0xa".into(),
                log_index: 2,
                price_usd: "".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let answers = ChainlinkAnswers {
            answers: vec![ChainlinkAnswer {
                tx_hash: "0xb".into(),
                log_index: 3,
                answer_usd: "1.500".into(),
                ..Default::default()
            }],
        };
        let ticks = BasisTicks {
            ticks: vec![BasisTick {
                tx_hash: "0xa".into(),
                log_index: 2,
                premium_bps: -7,
                ..Default::default()
            }],
        };
        let db = build(&clock(100), &swaps, &answers, &ticks);
        assert_eq!(db.table_changes.len(), 3);
        for c in &db.table_changes {
            assert_eq!(c.operation, Operation::Create as i32);
        }
        let swap = &changes_for(&db, "stock_swaps")[0];
        let field = |n: &str| swap.fields.iter().find(|f| f.name == n).map(|f| f.value.as_str());
        assert_eq!(field("price_usd_str"), Some("0"));
        assert!(field("price_usd").is_none());
        assert_eq!(field("priced"), Some("false"));
        let answer = &changes_for(&db, "chainlink_answers")[0];
        assert!(answer
            .fields
            .iter()
            .any(|f| f.name == "answer_usd_str" && f.value == "1.5"));
        let tick = &changes_for(&db, "basis_ticks")[0];
        assert!(tick.fields.iter().any(|f| f.name == "premium_bps" && f.value == "-7"));
    }

    #[test]
    fn dec_zeroes_out_values_beyond_the_decimal_bound() {
        assert_eq!(dec("123456789012345678901"), "0"); // 21 integer digits
        assert_eq!(dec("12345678901234567890"), "12345678901234567890"); // 20 is fine
        assert_eq!(dec("-123456789012345678901"), "0");
    }
}
