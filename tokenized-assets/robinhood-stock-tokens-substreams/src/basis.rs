//! Implied price vs reference. Reference precedence: Chainlink round, then the
//! last regular-session trade, else none.

use crate::pb::hood::basis::v1::{BasisTick, StockSwap};
use crate::price;
use crate::quality;
use crate::session;

pub const REF_PREFIX: &str = "ref:";
pub const CLOSE_PREFIX: &str = "close:";

pub fn ref_key(ticker: &str) -> String {
    format!("{REF_PREFIX}{ticker}")
}

pub fn close_key(ticker: &str) -> String {
    format!("{CLOSE_PREFIX}{ticker}")
}

/// "<answer_usd>|<updated_at>"
pub fn encode_ref(answer_usd: &str, updated_at: u64) -> String {
    format!("{answer_usd}|{updated_at}")
}

/// "<price_usd>|<block_ts>"
pub fn encode_close(price_usd: &str, block_ts: u64) -> String {
    format!("{price_usd}|{block_ts}")
}

struct Reference {
    usd: String,
    ts: u64,
    source: &'static str,
}

/// Parses a "<price>|<ts>" value written by encode_ref or encode_close.
fn parse_stored(v: &str, source: &'static str) -> Option<Reference> {
    let mut parts = v.split('|');
    let usd = parts.next()?.to_string();
    let ts = parts.next()?.parse().ok()?;
    Some(Reference { usd, ts, source })
}

pub fn tick(swap: &StockSwap, ref_value: Option<&str>, close_value: Option<&str>) -> Option<BasisTick> {
    if !quality::usable(swap) {
        return None;
    }
    let implied = price::parse(&swap.price_usd)?;
    let reference = ref_value
        .and_then(|v| parse_stored(v, "chainlink"))
        .or_else(|| close_value.and_then(|v| parse_stored(v, "session_close")));

    let (ref_usd, ref_ts, ref_source, premium_bps) = match reference {
        Some(r) => {
            let bps = price::parse(&r.usd)
                .and_then(|d| price::premium_bps(&implied, &d))
                .unwrap_or(0);
            (r.usd, r.ts, r.source, bps)
        }
        None => (String::new(), 0, "none", 0),
    };

    Some(BasisTick {
        block_num: swap.block_num,
        block_ts: swap.block_ts,
        tx_hash: swap.tx_hash.clone(),
        log_index: swap.log_index,
        ticker: swap.ticker.clone(),
        implied_usd: swap.price_usd.clone(),
        ref_usd,
        ref_source: ref_source.to_string(),
        ref_ts,
        premium_bps,
        session: session::classify(swap.block_ts).as_str().to_string(),
        amount_usd: swap.amount_usd.clone(),
        side: swap.side.clone(),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn swap(price_usd: &str) -> StockSwap {
        StockSwap {
            block_num: 1,
            block_ts: 1_781_706_600, // 2026-06-17 10:30 EDT, regular session
            tx_hash: "0xtx".into(),
            log_index: 9,
            ticker: "SGOV".into(),
            price_usd: price_usd.into(),
            amount_usd: "500".into(),
            side: "buy".into(),
            priced: true,
            shares_known: true,
            ..Default::default()
        }
    }

    #[test]
    fn chainlink_reference_wins() {
        let t = tick(&swap("101"), Some("100|1781706000"), Some("90|1781700000")).unwrap();
        assert_eq!(t.ref_source, "chainlink");
        assert_eq!(t.ref_usd, "100");
        assert_eq!(t.ref_ts, 1_781_706_000);
        assert_eq!(t.premium_bps, 100);
        assert_eq!(t.session, "regular");
        assert_eq!(t.implied_usd, "101");
        assert_eq!((t.amount_usd.as_str(), t.side.as_str()), ("500", "buy"));
    }

    #[test]
    fn falls_back_to_session_close() {
        let t = tick(&swap("99"), None, Some("100|1781700000")).unwrap();
        assert_eq!(t.ref_source, "session_close");
        assert_eq!(t.ref_ts, 1_781_700_000);
        assert_eq!(t.premium_bps, -100);
    }

    #[test]
    fn no_reference_yields_zero_premium() {
        let t = tick(&swap("99"), None, None).unwrap();
        assert_eq!(t.ref_source, "none");
        assert_eq!(t.ref_usd, "");
        assert_eq!(t.premium_bps, 0);
    }

    #[test]
    fn unpriced_swap_yields_no_tick() {
        assert!(tick(&swap(""), Some("100|1"), None).is_none());
        let mut s = swap("99");
        s.priced = false;
        assert!(tick(&s, Some("100|1"), None).is_none());
    }

    #[test]
    fn dust_and_mispriced_swaps_yield_no_tick() {
        let mut dust = swap("0.64");
        dust.amount_usd = "0.01".into();
        assert!(tick(&dust, Some("100|1"), None).is_none());
        let mut mispriced = swap("4600000000000");
        mispriced.amount_usd = "50000000000".into();
        assert!(tick(&mispriced, Some("100|1"), None).is_none());
    }

    #[test]
    fn malformed_store_values_are_ignored() {
        let t = tick(&swap("99"), Some("garbage"), Some("100|1781700000")).unwrap();
        assert_eq!(t.ref_source, "session_close");
        let t = tick(&swap("99"), Some("abc|1"), None).unwrap();
        assert_eq!(t.ref_source, "chainlink");
        assert_eq!(t.premium_bps, 0);
    }

    #[test]
    fn keys_and_encodings() {
        assert_eq!(ref_key("NVDA"), "ref:NVDA");
        assert_eq!(close_key("NVDA"), "close:NVDA");
        assert_eq!(encode_ref("1.5", 2), "1.5|2");
        assert_eq!(encode_close("1.5", 3), "1.5|3");
    }
}
