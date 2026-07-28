mod abi;
mod decode;
mod pb;

use substreams_ethereum::pb::eth::v2 as eth;

use crate::pb::uniswap::v4 as v4;

substreams_ethereum::init!();

/// Decode Uniswap v4 PoolManager + PositionManager logs (Base fixed addresses).
/// No Substreams stores: all HOL is projected in ClickHouse.
#[substreams::handlers::map]
fn map_events(blk: eth::Block) -> Result<v4::Events, substreams::errors::Error> {
    Ok(decode::decode_block(&blk))
}
