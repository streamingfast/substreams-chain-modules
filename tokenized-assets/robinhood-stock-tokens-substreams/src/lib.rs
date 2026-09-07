mod basis;
mod chainlink;
mod pb;
mod price;
mod registry;
mod rows;
mod session;
mod swaps;

use substreams::errors::Error;
use substreams::scalar::BigDecimal;
use substreams::store::{StoreGet, StoreGetString, StoreNew, StoreSet, StoreSetString};
use substreams_ethereum::pb::eth::v2::Block;

use pb::hood::basis::v1::{BasisTicks, ChainlinkAnswers, Rows, StockSwaps};
use pb::uniswap::v4::v1::Events;

#[substreams::handlers::map]
fn map_stock_swaps(events: Events) -> Result<StockSwaps, Error> {
    Ok(swaps::build(events))
}

#[substreams::handlers::map]
fn map_chainlink_answers(block: Block) -> Result<ChainlinkAnswers, Error> {
    Ok(chainlink::build(&block))
}

#[substreams::handlers::store]
fn store_ref_price(answers: ChainlinkAnswers, store: StoreSetString) {
    for a in &answers.answers {
        if a.ticker.is_empty() {
            continue;
        }
        let Some(answer_usd) = price::parse(&a.answer_usd) else {
            continue;
        };
        if answer_usd <= BigDecimal::zero() {
            continue;
        }
        store.set(
            a.log_index as u64,
            basis::ref_key(&a.ticker),
            &basis::encode_ref(&a.answer_usd, a.updated_at),
        );
    }
}

#[substreams::handlers::store]
fn store_session_close(swaps: StockSwaps, store: StoreSetString) {
    for s in &swaps.swaps {
        if s.ticker.is_empty() || s.price_usd.is_empty() || session::classify(s.block_ts) != session::Session::Regular {
            continue;
        }
        let Some(price_usd) = price::parse(&s.price_usd) else {
            continue;
        };
        if price_usd <= BigDecimal::zero() {
            continue;
        }
        store.set(
            s.log_index as u64,
            basis::close_key(&s.ticker),
            &basis::encode_close(&s.price_usd, s.block_ts),
        );
    }
}

#[substreams::handlers::map]
fn map_basis(swaps: StockSwaps, refs: StoreGetString, closes: StoreGetString) -> Result<BasisTicks, Error> {
    let mut out = BasisTicks::default();
    for s in &swaps.swaps {
        let r = refs.get_first(basis::ref_key(&s.ticker));
        let c = closes.get_first(basis::close_key(&s.ticker));
        if let Some(t) = basis::tick(s, r.as_deref(), c.as_deref()) {
            out.ticks.push(t);
        }
    }
    Ok(out)
}

#[substreams::handlers::map]
fn map_rows(swaps: StockSwaps, answers: ChainlinkAnswers, ticks: BasisTicks) -> Result<Rows, Error> {
    Ok(rows::build(swaps, answers, ticks))
}
