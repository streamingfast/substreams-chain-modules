//! Rows for substreams-sink-sql from-proto mode. Every emitted row gets its
//! `id` and `block_time` here, and every decimal column is rewritten to a
//! value a ClickHouse Decimal128(18) column accepts: the sink rejects empty
//! strings and values past 38 digits outright instead of nulling them.

use buffa_types::google::protobuf::Timestamp;

use crate::pb::hood::basis::v1::{BasisTicks, ChainlinkAnswers, Rows, StockSwaps};
use crate::price;

pub fn build(swaps: StockSwaps, answers: ChainlinkAnswers, ticks: BasisTicks) -> Rows {
    let mut out = Rows::default();

    for mut s in swaps.swaps {
        s.id = row_id(&s.tx_hash, s.log_index);
        s.block_time = block_time(s.block_ts).into();
        for v in [
            &mut s.shares_ui,
            &mut s.shares_raw_adjusted,
            &mut s.quote_amount,
            &mut s.amount_usd,
            &mut s.price_usd,
        ] {
            *v = price::decimal128(v);
        }
        out.stock_swaps.push(s);
    }

    for mut a in answers.answers {
        a.id = row_id(&a.tx_hash, a.log_index);
        a.block_time = block_time(a.block_ts).into();
        a.answer_usd = price::decimal128(&a.answer_usd);
        out.chainlink_answers.push(a);
    }

    for mut t in ticks.ticks {
        t.id = row_id(&t.tx_hash, t.log_index);
        t.block_time = block_time(t.block_ts).into();
        for v in [&mut t.implied_usd, &mut t.ref_usd, &mut t.amount_usd] {
            *v = price::decimal128(v);
        }
        out.basis_ticks.push(t);
    }

    out
}

pub fn row_id(tx_hash: &str, log_index: u32) -> String {
    format!("{tx_hash}-{log_index}")
}

fn block_time(block_ts: u64) -> Timestamp {
    Timestamp {
        seconds: block_ts as i64,
        nanos: 0,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pb::hood::basis::v1::{BasisTick, ChainlinkAnswer, StockSwap};

    #[test]
    fn every_row_gets_id_and_block_time() {
        let rows = build(
            StockSwaps {
                swaps: vec![StockSwap {
                    tx_hash: "0xa".into(),
                    log_index: 2,
                    block_ts: 1_781_706_600,
                    ..Default::default()
                }],
                ..Default::default()
            },
            ChainlinkAnswers {
                answers: vec![ChainlinkAnswer {
                    tx_hash: "0xb".into(),
                    log_index: 3,
                    block_ts: 1_781_706_601,
                    ..Default::default()
                }],
            },
            BasisTicks {
                ticks: vec![BasisTick {
                    tx_hash: "0xa".into(),
                    log_index: 2,
                    block_ts: 1_781_706_600,
                    ..Default::default()
                }],
            },
        );
        let s = &rows.stock_swaps[0];
        assert_eq!(s.id, "0xa-2");
        assert_eq!(
            s.block_time,
            Timestamp {
                seconds: 1_781_706_600,
                nanos: 0,
                ..Default::default()
            }
            .into()
        );
        let a = &rows.chainlink_answers[0];
        assert_eq!(a.id, "0xb-3");
        assert_eq!(a.block_time.as_ref().map(|t| t.seconds), Some(1_781_706_601));
        let t = &rows.basis_ticks[0];
        assert_eq!(t.id, s.id);
        assert_eq!(t.block_time, s.block_time);
    }

    #[test]
    fn decimals_are_never_empty_and_flags_survive() {
        let rows = build(
            StockSwaps {
                swaps: vec![StockSwap {
                    shares_ui: "12.500000".into(),
                    shares_raw_adjusted: "".into(),
                    quote_amount: "1e3".into(),
                    amount_usd: "".into(),
                    price_usd: "".into(),
                    priced: false,
                    shares_known: true,
                    ..Default::default()
                }],
                ..Default::default()
            },
            ChainlinkAnswers {
                answers: vec![ChainlinkAnswer {
                    answer_usd: "-0.10".into(),
                    ..Default::default()
                }],
            },
            BasisTicks {
                ticks: vec![BasisTick {
                    implied_usd: "101".into(),
                    ref_usd: "".into(),
                    ref_source: "none".into(),
                    amount_usd: "123456789012345678901".into(),
                    ..Default::default()
                }],
            },
        );
        let s = &rows.stock_swaps[0];
        assert_eq!(s.shares_ui, "12.5");
        assert_eq!(s.shares_raw_adjusted, "0");
        assert_eq!(s.quote_amount, "1000");
        assert_eq!((s.amount_usd.as_str(), s.price_usd.as_str()), ("0", "0"));
        assert!(!s.priced);
        assert!(s.shares_known);
        assert_eq!(rows.chainlink_answers[0].answer_usd, "-0.1");
        let t = &rows.basis_ticks[0];
        assert_eq!((t.implied_usd.as_str(), t.ref_usd.as_str()), ("101", "0"));
        assert_eq!(t.amount_usd, "0"); // 21 integer digits do not fit Decimal128(18)
    }

    #[test]
    fn row_id_joins_tx_hash_and_log_index() {
        assert_eq!(row_id("0xdeadbeef", 0), "0xdeadbeef-0");
        assert_eq!(row_id("0xdeadbeef", 42), "0xdeadbeef-42");
    }
}
