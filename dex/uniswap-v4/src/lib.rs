mod abi;
mod decode;
mod pb;

use substreams_ethereum::pb::eth::v2 as eth;

use crate::pb::uniswap::v4 as v4;

substreams_ethereum::init!();

/// Decode Uniswap v4 PoolManager + PositionManager logs. Contract addresses come
/// from `params` (`pool_manager=0x...&position_manager=0x...`), set per network in
/// the manifest.
/// No Substreams stores: all HOL is projected in ClickHouse.
#[substreams::handlers::map]
fn map_events(params: String, blk: eth::Block) -> Result<v4::Events, substreams::errors::Error> {
    let config = decode::Config::parse(&params)?;
    Ok(decode::decode_block(&config, &blk))
}
