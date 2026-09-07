// Baked registry snapshot. Some fields are only read by the schema generator and tests.
#![allow(dead_code)]
//! Static Robinhood stock-token registry and Chainlink feed map, baked from
//! data/*.tsv by build.rs. Addresses are 0x-prefixed lowercase.

pub struct Asset {
    pub ticker: &'static str,
    pub token: &'static str,
    pub multiplier: &'static str,
    pub decimals: u32,
    pub status: &'static str,
    pub name: &'static str,
}

pub struct Feed {
    pub ticker: &'static str,
    pub token: &'static str,
    pub proxy: &'static str,
    pub aggregator: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/registry_data.rs"));

pub const USDG: &str = "0x5fc5360d0400a0fd4f2af552add042d716f1d168";
pub const WETH: &str = "0x0bd7d308f8e1639fab988df18a8011f41eacad73";
pub const NATIVE: &str = "0x0000000000000000000000000000000000000000";

pub fn asset_by_token(token: &str) -> Option<&'static Asset> {
    ASSETS.binary_search_by(|a| a.token.cmp(token)).ok().map(|i| &ASSETS[i])
}

pub fn feed_by_aggregator(aggregator: &str) -> Option<&'static Feed> {
    FEEDS
        .binary_search_by(|f| f.aggregator.cmp(aggregator))
        .ok()
        .map(|i| &FEEDS[i])
}

pub fn feed_by_token(token: &str) -> Option<&'static Feed> {
    FEEDS.iter().find(|f| f.token == token)
}

pub fn quote_kind(token: &str) -> Option<&'static str> {
    match token {
        USDG => Some("usdg"),
        WETH => Some("weth"),
        NATIVE => Some("native"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_loaded_and_sorted() {
        assert_eq!(ASSETS.len(), 194);
        assert_eq!(FEEDS.len(), 35);
        assert!(ASSETS.windows(2).all(|w| w[0].token < w[1].token));
        assert!(FEEDS.windows(2).all(|w| w[0].aggregator < w[1].aggregator));
    }

    #[test]
    fn every_feed_token_is_a_registry_asset_with_same_ticker() {
        for f in FEEDS {
            let a = asset_by_token(f.token).unwrap_or_else(|| panic!("{} not in registry", f.ticker));
            assert_eq!(a.ticker, f.ticker);
            assert_eq!(feed_by_aggregator(f.aggregator).unwrap().ticker, f.ticker);
            assert_eq!(feed_by_token(f.token).unwrap().proxy, f.proxy);
        }
    }

    #[test]
    fn known_lookups() {
        let sgov = asset_by_token("0x92fd66527192e3e61d4ddd13322aa222de86f9b5").unwrap();
        assert_eq!(sgov.ticker, "SGOV");
        assert_eq!(sgov.decimals, 18);
        assert_eq!(
            feed_by_aggregator("0x0e96b7708487f91baac09697593d3e8bf253f2d8")
                .unwrap()
                .ticker,
            "SGOV"
        );
        assert!(asset_by_token(USDG).is_none());
        assert_eq!(quote_kind(USDG), Some("usdg"));
        assert_eq!(quote_kind(WETH), Some("weth"));
        assert_eq!(quote_kind(NATIVE), Some("native"));
        assert_eq!(quote_kind("0x0ff7a7420000000000000000000000000000dead"), None);
    }
}
