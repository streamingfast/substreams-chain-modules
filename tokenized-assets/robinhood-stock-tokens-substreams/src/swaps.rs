//! Re-frames uniswap-v4-robinhood stock swaps as share trades against a quote.

use std::str::FromStr;
use substreams::scalar::BigInt;

use crate::pb::hood::basis::v1::{StockSwap, StockSwaps};
use crate::pb::uniswap::v4::v1::{Events, Swap};
use crate::price;
use crate::registry;

pub fn build(events: Events) -> StockSwaps {
    let mut out = StockSwaps::default();
    for swap in events.swaps {
        match convert(&swap) {
            Ok(row) => out.swaps.push(row),
            Err(Skip::StockStock) => out.skipped_stock_stock += 1,
            Err(Skip::Other) => out.skipped_other += 1,
        }
    }
    out
}

enum Skip {
    StockStock,
    Other,
}

struct Leg<'a> {
    token: &'a str,
    symbol: &'a str,
    raw: &'a str,
    adjusted: &'a str,
    ui: &'a str,
}

fn convert(swap: &Swap) -> Result<StockSwap, Skip> {
    let leg0 = Leg {
        token: &swap.token0,
        symbol: &swap.token0_symbol,
        raw: &swap.amount0,
        adjusted: &swap.amount0_adjusted,
        ui: &swap.amount0_ui,
    };
    let leg1 = Leg {
        token: &swap.token1,
        symbol: &swap.token1_symbol,
        raw: &swap.amount1,
        adjusted: &swap.amount1_adjusted,
        ui: &swap.amount1_ui,
    };
    let (stock, quote) = match (swap.token0_is_stock, swap.token1_is_stock) {
        (true, false) => (leg0, leg1),
        (false, true) => (leg1, leg0),
        (true, true) => return Err(Skip::StockStock),
        (false, false) => return Err(Skip::Other),
    };
    let quote_kind = registry::quote_kind(quote.token).ok_or(Skip::Other)?;

    let ticker = registry::asset_by_token(stock.token)
        .map(|a| a.ticker.to_string())
        .unwrap_or_else(|| swap.registry_symbol.clone());

    // Raw amounts are swapper-centric: positive means the swapper received it.
    let received_stock = BigInt::from_str(stock.raw).map(|v| v > BigInt::zero()).unwrap_or(false);

    let shares_ui = price::abs_str(stock.ui);
    let amount_usd = if swap.priced {
        swap.amount_usd.clone()
    } else {
        String::new()
    };
    let price_usd = price::price_usd(&amount_usd, &shares_ui).unwrap_or_default();

    let quote_symbol = if quote.symbol.is_empty() {
        match quote_kind {
            "usdg" => "USDG",
            "weth" => "WETH",
            _ => "ETH",
        }
        .to_string()
    } else {
        quote.symbol.to_string()
    };

    let meta = swap.meta.as_ref();
    Ok(StockSwap {
        block_num: meta.map(|m| m.block_number).unwrap_or_default(),
        block_ts: meta.map(|m| m.block_timestamp).unwrap_or_default(),
        tx_hash: meta.map(|m| m.tx_hash.clone()).unwrap_or_default(),
        log_index: meta.map(|m| m.log_index).unwrap_or_default(),
        pool_id: swap.pool_id.clone(),
        ticker,
        token: stock.token.to_string(),
        side: if received_stock { "buy" } else { "sell" }.to_string(),
        shares_ui,
        shares_raw_adjusted: price::abs_str(stock.adjusted),
        quote_symbol,
        quote_kind: quote_kind.to_string(),
        quote_amount: price::abs_str(quote.adjusted),
        amount_usd,
        price_usd,
        sender: swap.sender.clone(),
        origin: meta.map(|m| m.origin.clone()).unwrap_or_default(),
        fee: swap.fee,
        hook_address: swap.hook.as_ref().map(|h| h.address.clone()).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pb::uniswap::v4::v1::{HookPermissions, Meta};

    const SGOV: &str = "0x92fd66527192e3e61d4ddd13322aa222de86f9b5";

    fn base_swap() -> Swap {
        Swap {
            pool_id: "0xpool".into(),
            sender: "0xsender".into(),
            fee: 500,
            meta: Some(Meta {
                block_number: 52_700_001,
                block_timestamp: 1_781_706_600,
                tx_hash: "0xtx".into(),
                log_index: 3,
                origin: "0xorigin".into(),
                ..Default::default()
            }),
            hook: Some(HookPermissions {
                address: "0xhook".into(),
                ..Default::default()
            }),
            priced: true,
            amounts_adjusted: true,
            ..Default::default()
        }
    }

    // Trader sells 2 SGOV (raw negative on the stock leg) for 200 USDG.
    fn sgov_usdg_sell() -> Swap {
        let mut s = base_swap();
        s.token0 = SGOV.into();
        s.token1 = registry::USDG.into();
        s.token0_symbol = "SGOV".into();
        s.token1_symbol = "USDG".into();
        s.token0_is_stock = true;
        s.registry_symbol = "SGOV".into();
        s.amount0 = "-2000000000000000000".into();
        s.amount1 = "200000000".into();
        s.amount0_adjusted = "2".into();
        s.amount1_adjusted = "-200".into();
        s.amount0_ui = "2.000000000000000000".into();
        s.amount_usd = "200".into();
        s
    }

    #[test]
    fn sell_against_usdg() {
        let out = build(Events {
            swaps: vec![sgov_usdg_sell()],
            ..Default::default()
        });
        assert_eq!(out.swaps.len(), 1);
        let r = &out.swaps[0];
        assert_eq!(r.ticker, "SGOV");
        assert_eq!(r.token, SGOV);
        assert_eq!(r.side, "sell");
        assert_eq!(r.shares_ui, "2.000000000000000000");
        assert_eq!(r.shares_raw_adjusted, "2");
        assert_eq!(r.quote_symbol, "USDG");
        assert_eq!(r.quote_kind, "usdg");
        assert_eq!(r.quote_amount, "200");
        assert_eq!(r.amount_usd, "200");
        assert_eq!(r.price_usd, "100");
        assert_eq!(r.hook_address, "0xhook");
        assert_eq!((r.block_num, r.block_ts, r.log_index), (52_700_001, 1_781_706_600, 3));
        assert_eq!(
            (r.tx_hash.as_str(), r.origin.as_str(), r.sender.as_str()),
            ("0xtx", "0xorigin", "0xsender")
        );
    }

    #[test]
    fn buy_with_stock_on_leg1_and_native_quote() {
        let mut s = base_swap();
        s.token0 = registry::NATIVE.into();
        s.token1 = SGOV.into();
        s.token1_is_stock = true;
        s.amount0 = "-1000000000000000000".into();
        s.amount1 = "4000000000000000000".into();
        s.amount0_adjusted = "1".into();
        s.amount1_adjusted = "-4".into();
        s.amount1_ui = "-4".into();
        s.amount_usd = "3000".into();
        let out = build(Events {
            swaps: vec![s],
            ..Default::default()
        });
        let r = &out.swaps[0];
        assert_eq!(r.side, "buy");
        assert_eq!(r.quote_kind, "native");
        assert_eq!(r.quote_symbol, "ETH");
        assert_eq!(r.quote_amount, "1");
        assert_eq!(r.shares_ui, "4");
        assert_eq!(r.price_usd, "750");
    }

    #[test]
    fn unpriced_swap_has_empty_usd_fields() {
        let mut s = sgov_usdg_sell();
        s.priced = false;
        let out = build(Events {
            swaps: vec![s],
            ..Default::default()
        });
        let r = &out.swaps[0];
        assert_eq!(r.amount_usd, "");
        assert_eq!(r.price_usd, "");
        assert_eq!(r.shares_ui, "2.000000000000000000");
    }

    #[test]
    fn skips_stock_stock_and_unknown_quotes() {
        let mut ss = sgov_usdg_sell();
        ss.token1_is_stock = true;
        let mut meme = sgov_usdg_sell();
        meme.token1 = "0x0ff7a7420000000000000000000000000000dead".into();
        let mut none = sgov_usdg_sell();
        none.token0_is_stock = false;
        let out = build(Events {
            swaps: vec![ss, meme, none],
            ..Default::default()
        });
        assert!(out.swaps.is_empty());
        assert_eq!(out.skipped_stock_stock, 1);
        assert_eq!(out.skipped_other, 2);
    }
}
