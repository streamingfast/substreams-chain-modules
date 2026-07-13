mod abi;
mod pb;

use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::hyperlock_finance::types::v1::{
    Events, Erc20PointsDepositStake, Erc20PointsDepositUnstake, Erc721PointsDepositDeposit, Erc721PointsDepositWithdraw, ThrusterPointNftDecreaseLiquidity, ThrusterPointNftIncreaseLiquidity,
};

const THRUSTER_POINT_NFT: [u8; 20] = hex_literal::hex!("434575eaea081b735c985fa9bf63cd7b87e227f9");
const ERC20_POINTS_DEPOSIT: [u8; 20] = hex_literal::hex!("c3ecadb7a5fab07c72af6bcfbd588b7818c4a40e");
const ERC721_POINTS_DEPOSIT: [u8; 20] = hex_literal::hex!("c28effdfef75448243c1d9ba972b97e32df60d06");

fn fmt_addr(addr: &[u8]) -> String {
    format!("0x{}", hex::encode(addr))
}

fn block_timestamp(block: &Block) -> u64 {
    block
        .header
        .as_ref()
        .and_then(|h| h.timestamp.as_ref().map(|t| t.seconds as u64))
        .unwrap_or(0)
}

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == THRUSTER_POINT_NFT.as_slice() {
                if let Some(ev) =
                    abi::thruster_point_nft::events::IncreaseLiquidity::match_and_decode(log)
                {
                    events.thruster_point_nft_increase_liquiditys.push(ThrusterPointNftIncreaseLiquidity {
                        id: id.clone(),
                        token_id: ev.token_id.to_string(),
                        liquidity: ev.liquidity.to_string(),
                        amount0: ev.amount0.to_string(),
                        amount1: ev.amount1.to_string(),
                        tick_lower: ev.tick_lower.to_string(),
                        tick_upper: ev.tick_upper.to_string(),
                        pool: fmt_addr(&ev.pool),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::thruster_point_nft::events::DecreaseLiquidity::match_and_decode(log)
                {
                    events.thruster_point_nft_decrease_liquiditys.push(ThrusterPointNftDecreaseLiquidity {
                        id: id.clone(),
                        token_id: ev.token_id.to_string(),
                        liquidity: ev.liquidity.to_string(),
                        amount0: ev.amount0.to_string(),
                        amount1: ev.amount1.to_string(),
                        tick_lower: ev.tick_lower.to_string(),
                        tick_upper: ev.tick_upper.to_string(),
                        pool: fmt_addr(&ev.pool),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == ERC20_POINTS_DEPOSIT.as_slice() {
                if let Some(ev) =
                    abi::erc20_points_deposit::events::Stake::match_and_decode(log)
                {
                    events.erc20_points_deposit_stakes.push(Erc20PointsDepositStake {
                        id: id.clone(),
                        lp_token: fmt_addr(&ev.lp_token),
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::erc20_points_deposit::events::Unstake::match_and_decode(log)
                {
                    events.erc20_points_deposit_unstakes.push(Erc20PointsDepositUnstake {
                        id: id.clone(),
                        lp_token: fmt_addr(&ev.lp_token),
                        sender: fmt_addr(&ev.sender),
                        amount: ev.amount.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == ERC721_POINTS_DEPOSIT.as_slice() {
                if let Some(ev) =
                    abi::erc721_points_deposit::events::Deposit::match_and_decode(log)
                {
                    events.erc721_points_deposit_deposits.push(Erc721PointsDepositDeposit {
                        id: id.clone(),
                        pool: fmt_addr(&ev.pool),
                        sender: fmt_addr(&ev.sender),
                        token_id: ev.token_id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::erc721_points_deposit::events::Withdraw::match_and_decode(log)
                {
                    events.erc721_points_deposit_withdraws.push(Erc721PointsDepositWithdraw {
                        id: id.clone(),
                        pool: fmt_addr(&ev.pool),
                        sender: fmt_addr(&ev.sender),
                        token_id: ev.token_id.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn db_out(events: Events) -> Result<DatabaseChanges, Error> {
    let mut tables = Tables::new();

    for e in events.thruster_point_nft_increase_liquiditys {
        tables
            .create_row("thruster_point_nft_increase_liquidity", &e.id)
            .set("token_id", &e.token_id)
            .set("liquidity", &e.liquidity)
            .set("amount0", &e.amount0)
            .set("amount1", &e.amount1)
            .set("tick_lower", &e.tick_lower)
            .set("tick_upper", &e.tick_upper)
            .set("pool", &e.pool)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.thruster_point_nft_decrease_liquiditys {
        tables
            .create_row("thruster_point_nft_decrease_liquidity", &e.id)
            .set("token_id", &e.token_id)
            .set("liquidity", &e.liquidity)
            .set("amount0", &e.amount0)
            .set("amount1", &e.amount1)
            .set("tick_lower", &e.tick_lower)
            .set("tick_upper", &e.tick_upper)
            .set("pool", &e.pool)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.erc20_points_deposit_stakes {
        tables
            .create_row("erc20_points_deposit_stake", &e.id)
            .set("lp_token", &e.lp_token)
            .set("sender", &e.sender)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.erc20_points_deposit_unstakes {
        tables
            .create_row("erc20_points_deposit_unstake", &e.id)
            .set("lp_token", &e.lp_token)
            .set("sender", &e.sender)
            .set("amount", &e.amount)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.erc721_points_deposit_deposits {
        tables
            .create_row("erc721_points_deposit_deposit", &e.id)
            .set("pool", &e.pool)
            .set("sender", &e.sender)
            .set("token_id", &e.token_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.erc721_points_deposit_withdraws {
        tables
            .create_row("erc721_points_deposit_withdraw", &e.id)
            .set("pool", &e.pool)
            .set("sender", &e.sender)
            .set("token_id", &e.token_id)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
