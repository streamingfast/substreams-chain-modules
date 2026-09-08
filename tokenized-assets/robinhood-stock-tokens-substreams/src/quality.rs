//! Data-quality floors. A swap is usable as a price observation only when it
//! is priced, its share count is known, and both notional and implied price
//! fall inside bounds that exclude dust trades and upstream mispricing. Only
//! usable swaps feed the session-close store and basis ticks; the raw tape in
//! map_stock_swaps keeps every swap and carries the verdict in `usable`.

use substreams::scalar::BigDecimal;

use crate::pb::hood::basis::v1::StockSwap;
use crate::price;

pub const MIN_NOTIONAL_USD: &str = "5";
pub const MAX_NOTIONAL_USD: &str = "10000000";
pub const MIN_PRICE_USD: &str = "0.01";
pub const MAX_PRICE_USD: &str = "100000";

pub fn usable(swap: &StockSwap) -> bool {
    if !swap.priced || !swap.shares_known {
        return false;
    }
    let (Some(amount_usd), Some(price_usd)) = (price::parse(&swap.amount_usd), price::parse(&swap.price_usd)) else {
        return false;
    };
    within(&amount_usd, MIN_NOTIONAL_USD, MAX_NOTIONAL_USD) && within(&price_usd, MIN_PRICE_USD, MAX_PRICE_USD)
}

fn within(v: &BigDecimal, min: &str, max: &str) -> bool {
    let min = price::parse(min).expect("bound is a valid decimal");
    let max = price::parse(max).expect("bound is a valid decimal");
    *v >= min && *v <= max
}

#[cfg(test)]
mod tests {
    use super::*;

    fn swap(amount_usd: &str, price_usd: &str) -> StockSwap {
        StockSwap {
            amount_usd: amount_usd.into(),
            price_usd: price_usd.into(),
            priced: true,
            shares_known: true,
            ..Default::default()
        }
    }

    #[test]
    fn typical_trade_is_usable() {
        assert!(usable(&swap("1500", "150")));
    }

    #[test]
    fn requires_priced_and_shares_known() {
        let mut s = swap("1500", "150");
        s.priced = false;
        assert!(!usable(&s));
        let mut s = swap("1500", "150");
        s.shares_known = false;
        assert!(!usable(&s));
    }

    #[test]
    fn unparsable_values_are_unusable() {
        assert!(!usable(&swap("", "150")));
        assert!(!usable(&swap("1500", "")));
        assert!(!usable(&swap("abc", "150")));
    }

    #[test]
    fn notional_floor() {
        assert!(usable(&swap("5", "150")));
        assert!(!usable(&swap("4.999999999999999999", "150")));
        assert!(!usable(&swap("0.01", "0.64")));
    }

    #[test]
    fn notional_ceiling() {
        assert!(usable(&swap("10000000", "150")));
        assert!(!usable(&swap("10000000.000000000000000001", "150")));
        assert!(!usable(&swap("50000000000", "4600000000000")));
    }

    #[test]
    fn price_floor() {
        assert!(usable(&swap("100", "0.01")));
        assert!(!usable(&swap("100", "0.009999999999999999")));
        assert!(!usable(&swap("100", "0")));
    }

    #[test]
    fn price_ceiling() {
        assert!(usable(&swap("100", "100000")));
        assert!(!usable(&swap("100", "100000.000000000000000001")));
    }
}
