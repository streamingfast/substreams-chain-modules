//! Exact decimal arithmetic for prices. Never f64.

use std::str::FromStr;
use substreams::scalar::{BigDecimal, BigInt};

const SCALE: i64 = 18;

pub fn parse(s: &str) -> Option<BigDecimal> {
    if s.is_empty() {
        return None;
    }
    BigDecimal::from_str(s).ok()
}

/// Strip a leading sign from a decimal string without re-formatting it.
pub fn abs_str(s: &str) -> String {
    s.strip_prefix('-').unwrap_or(s).to_string()
}

/// Truncate to 18 decimals and drop trailing zeros. Formats from the scaled
/// integer because bigdecimal's Display switches to scientific notation for
/// small magnitudes.
pub fn fmt(d: &BigDecimal) -> String {
    let scaled = (d.clone() * BigDecimal::new(BigInt::one(), SCALE)).to_bigint();
    let negative = scaled.to_string().starts_with('-');
    let (int_part, frac_part) = scaled.absolute().div_rem(&BigInt::from(10u64).pow(SCALE as u32));
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&int_part.to_string());
    let frac = format!("{:0>width$}", frac_part.to_string(), width = SCALE as usize);
    let frac = frac.trim_end_matches('0');
    if !frac.is_empty() {
        out.push('.');
        out.push_str(frac);
    }
    if out == "-0" {
        out = "0".to_string();
    }
    out
}

/// Decimal string for a ClickHouse Decimal128(18) column: truncated to 18
/// decimals, "0" when empty or unparsable, and "0" past 20 integer digits
/// (38 - 18) because the sink rejects a value that does not fit rather than
/// nulling it out.
pub fn decimal128(s: &str) -> String {
    let out = parse(s).map(|d| fmt(&d)).unwrap_or_else(|| "0".to_string());
    if int_digits(&out) > 20 {
        return "0".to_string();
    }
    out
}

fn int_digits(s: &str) -> usize {
    let s = s.strip_prefix('-').unwrap_or(s);
    s.split('.').next().unwrap_or(s).len()
}

/// amount_usd / shares; None when either is unparsable or shares is zero.
pub fn price_usd(amount_usd: &str, shares: &str) -> Option<String> {
    let a = parse(amount_usd)?;
    let s = parse(shares)?;
    if s.is_zero() {
        return None;
    }
    Some(fmt(&(a / s)))
}

/// (implied / reference - 1) * 10000, truncated toward zero. Saturates to
/// i64::MAX / i64::MIN on overflow; None only for a zero/invalid reference.
pub fn premium_bps(implied: &BigDecimal, reference: &BigDecimal) -> Option<i64> {
    if reference.is_zero() {
        return None;
    }
    let ratio = implied.clone() / reference.clone();
    let bps = (ratio - BigDecimal::one()) * BigDecimal::from(10_000);
    let digits = bps.to_bigint().to_string();
    Some(i64::from_str(&digits).unwrap_or(if digits.starts_with('-') { i64::MIN } else { i64::MAX }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> BigDecimal {
        parse(s).unwrap()
    }

    #[test]
    fn fmt_truncates_and_trims() {
        assert_eq!(fmt(&dec("150")), "150");
        assert_eq!(fmt(&dec("150.500000")), "150.5");
        assert_eq!(fmt(&dec("0")), "0");
        assert_eq!(fmt(&dec("-0.25")), "-0.25");
        assert_eq!(fmt(&dec("1.1234567890123456789999")), "1.123456789012345678");
        assert_eq!(fmt(&(dec("1") / dec("3"))), "0.333333333333333333");
    }

    #[test]
    fn abs_str_only_strips_sign() {
        assert_eq!(abs_str("-12.500000"), "12.500000");
        assert_eq!(abs_str("12.5"), "12.5");
        assert_eq!(abs_str(""), "");
    }

    #[test]
    fn price_from_notional_and_shares() {
        assert_eq!(price_usd("1500", "10").as_deref(), Some("150"));
        assert_eq!(price_usd("1234.5678", "2.5").as_deref(), Some("493.82712"));
        assert_eq!(price_usd("1500", "0"), None);
        assert_eq!(price_usd("", "10"), None);
        assert_eq!(price_usd("1500", ""), None);
        assert_eq!(price_usd("abc", "10"), None);
    }

    #[test]
    fn premium_in_basis_points() {
        assert_eq!(premium_bps(&dec("101"), &dec("100")), Some(100));
        assert_eq!(premium_bps(&dec("99.5"), &dec("100")), Some(-50));
        assert_eq!(premium_bps(&dec("100"), &dec("100")), Some(0));
        assert_eq!(premium_bps(&dec("100.004"), &dec("100")), Some(0)); // truncates 0.4 bps
        assert_eq!(premium_bps(&dec("200"), &dec("100")), Some(10_000));
        assert_eq!(premium_bps(&dec("100"), &dec("0")), None);
    }

    #[test]
    fn premium_saturates_on_overflow() {
        let huge = dec("100000000000000000000000000");
        assert_eq!(premium_bps(&huge, &dec("1")), Some(i64::MAX));
        assert_eq!(premium_bps(&(dec("0") - huge), &dec("1")), Some(i64::MIN));
    }

    #[test]
    fn decimal128_is_always_a_valid_bounded_decimal() {
        assert_eq!(decimal128(""), "0");
        assert_eq!(decimal128("abc"), "0");
        assert_eq!(decimal128("150.500000"), "150.5");
        assert_eq!(decimal128("-0.25"), "-0.25");
        assert_eq!(decimal128("1.1234567890123456789999"), "1.123456789012345678");
        assert_eq!(decimal128("12345678901234567890"), "12345678901234567890"); // 20 integer digits fit
        assert_eq!(decimal128("123456789012345678901"), "0"); // 21 do not
        assert_eq!(decimal128("-123456789012345678901"), "0");
    }
}
