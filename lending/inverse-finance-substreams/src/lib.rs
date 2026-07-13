mod abi;
mod pb;

use substreams::errors::Error;
use substreams::store::{StoreGet, StoreGetString, StoreNew, StoreSet, StoreSetString};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

use crate::pb::inverse_finance::types::v1::{
    Events, CTokenAccrueInterest, CTokenBorrow, CTokenLiquidateBorrow, CTokenMint, CTokenRedeem, CTokenRepayBorrow, DolaTransfer, FactoryActionPaused, FactoryDistributedBorrowerComp, FactoryDistributedSupplierComp, FactoryMarketListed, FactoryNewCloseFactor, FactoryNewCollateralFactor, FactoryNewLiquidationIncentive, InvOwnerChanged, StablizerBuy, StablizerSell,
};

const FACTORY: [u8; 20] = hex_literal::hex!("4dcf7407ae5c07f8681e1659f626e114a7667339");
const INV: [u8; 20] = hex_literal::hex!("41d5d79431a913c4ae7d69a668ecdfe5ff9dfb68");
const DOLA: [u8; 20] = hex_literal::hex!("865377367054516e17014ccded1e7d814edc9ce4");
const STABLIZER: [u8; 20] = hex_literal::hex!("7ec0d931affba01b77711c2cd07c76b970795cdd");

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

#[substreams::handlers::store]
pub fn store_pools(block: Block, store: StoreSetString) {
    for trx in block.transactions() {
        for log in trx.receipt().logs() {
            if log.address() == FACTORY.as_slice() {
                if let Some(ev) =
                    abi::factory::events::MarketListed::match_and_decode(log)
                {
                    store.set(log.ordinal(), fmt_addr(&ev.c_token), &"1".to_string());
                }
            }
        }
    }
}

#[substreams::handlers::map]
pub fn map_events(block: Block, store: StoreGetString) -> Result<Events, Error> {
    let mut events = Events::default();
    let timestamp = block_timestamp(&block);

    for trx in block.transactions() {
        let tx_hash = format!("0x{}", hex::encode(&trx.hash));

        for log in trx.receipt().logs() {
            let id = format!("{}-{}", tx_hash, log.index());

            if log.address() == FACTORY.as_slice() {
                if let Some(ev) =
                    abi::factory::events::MarketListed::match_and_decode(log)
                {
                    events.factory_market_listeds.push(FactoryMarketListed {
                        id: id.clone(),
                        c_token: fmt_addr(&ev.c_token),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::factory::events::ActionPaused::match_and_decode(log)
                {
                    events.factory_action_pauseds.push(FactoryActionPaused {
                        id: id.clone(),
                        c_token: fmt_addr(&ev.c_token),
                        action: ev.action,
                        pause_state: ev.pause_state.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::factory::events::DistributedBorrowerComp::match_and_decode(log)
                {
                    events.factory_distributed_borrower_comps.push(FactoryDistributedBorrowerComp {
                        id: id.clone(),
                        c_token: fmt_addr(&ev.c_token),
                        borrower: fmt_addr(&ev.borrower),
                        comp_delta: ev.comp_delta.to_string(),
                        comp_borrow_index: ev.comp_borrow_index.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::factory::events::DistributedSupplierComp::match_and_decode(log)
                {
                    events.factory_distributed_supplier_comps.push(FactoryDistributedSupplierComp {
                        id: id.clone(),
                        c_token: fmt_addr(&ev.c_token),
                        supplier: fmt_addr(&ev.supplier),
                        comp_delta: ev.comp_delta.to_string(),
                        comp_supply_index: ev.comp_supply_index.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::factory::events::NewCollateralFactor::match_and_decode(log)
                {
                    events.factory_new_collateral_factors.push(FactoryNewCollateralFactor {
                        id: id.clone(),
                        c_token: fmt_addr(&ev.c_token),
                        old_collateral_factor_mantissa: ev.old_collateral_factor_mantissa.to_string(),
                        new_collateral_factor_mantissa: ev.new_collateral_factor_mantissa.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::factory::events::NewCloseFactor::match_and_decode(log)
                {
                    events.factory_new_close_factors.push(FactoryNewCloseFactor {
                        id: id.clone(),
                        old_close_factor_mantissa: ev.old_close_factor_mantissa.to_string(),
                        new_close_factor_mantissa: ev.new_close_factor_mantissa.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::factory::events::NewLiquidationIncentive::match_and_decode(log)
                {
                    events.factory_new_liquidation_incentives.push(FactoryNewLiquidationIncentive {
                        id: id.clone(),
                        old_liquidation_incentive_mantissa: ev.old_liquidation_incentive_mantissa.to_string(),
                        new_liquidation_incentive_mantissa: ev.new_liquidation_incentive_mantissa.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if store.get_last(fmt_addr(log.address())).is_some() {
                let pool = fmt_addr(log.address());
                if let Some(ev) =
                    abi::c_token::events::Mint::match_and_decode(log)
                {
                    events.c_token_mints.push(CTokenMint {
                        id: id.clone(),
                        pool: pool.clone(),
                        minter: fmt_addr(&ev.minter),
                        mint_amount: ev.mint_amount.to_string(),
                        mint_tokens: ev.mint_tokens.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::c_token::events::Redeem::match_and_decode(log)
                {
                    events.c_token_redeems.push(CTokenRedeem {
                        id: id.clone(),
                        pool: pool.clone(),
                        redeemer: fmt_addr(&ev.redeemer),
                        redeem_amount: ev.redeem_amount.to_string(),
                        redeem_tokens: ev.redeem_tokens.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::c_token::events::Borrow::match_and_decode(log)
                {
                    events.c_token_borrows.push(CTokenBorrow {
                        id: id.clone(),
                        pool: pool.clone(),
                        borrower: fmt_addr(&ev.borrower),
                        borrow_amount: ev.borrow_amount.to_string(),
                        account_borrows: ev.account_borrows.to_string(),
                        total_borrows: ev.total_borrows.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::c_token::events::RepayBorrow::match_and_decode(log)
                {
                    events.c_token_repay_borrows.push(CTokenRepayBorrow {
                        id: id.clone(),
                        pool: pool.clone(),
                        payer: fmt_addr(&ev.payer),
                        borrower: fmt_addr(&ev.borrower),
                        repay_amount: ev.repay_amount.to_string(),
                        account_borrows: ev.account_borrows.to_string(),
                        total_borrows: ev.total_borrows.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::c_token::events::LiquidateBorrow::match_and_decode(log)
                {
                    events.c_token_liquidate_borrows.push(CTokenLiquidateBorrow {
                        id: id.clone(),
                        pool: pool.clone(),
                        liquidator: fmt_addr(&ev.liquidator),
                        borrower: fmt_addr(&ev.borrower),
                        repay_amount: ev.repay_amount.to_string(),
                        c_token_collateral: fmt_addr(&ev.c_token_collateral),
                        seize_tokens: ev.seize_tokens.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::c_token::events::AccrueInterest::match_and_decode(log)
                {
                    events.c_token_accrue_interests.push(CTokenAccrueInterest {
                        id: id.clone(),
                        pool: pool.clone(),
                        cash_prior: ev.cash_prior.to_string(),
                        interest_accumulated: ev.interest_accumulated.to_string(),
                        borrow_index: ev.borrow_index.to_string(),
                        total_borrows: ev.total_borrows.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == INV.as_slice() {
                if let Some(ev) =
                    abi::inv::events::OwnerChanged::match_and_decode(log)
                {
                    events.inv_owner_changeds.push(InvOwnerChanged {
                        id: id.clone(),
                        owner: fmt_addr(&ev.owner),
                        new_owner: fmt_addr(&ev.new_owner),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == DOLA.as_slice() {
                if let Some(ev) =
                    abi::dola::events::Transfer::match_and_decode(log)
                {
                    events.dola_transfers.push(DolaTransfer {
                        id: id.clone(),
                        from: fmt_addr(&ev.from),
                        to: fmt_addr(&ev.to),
                        value: ev.value.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
            }

            if log.address() == STABLIZER.as_slice() {
                if let Some(ev) =
                    abi::stablizer::events::Sell::match_and_decode(log)
                {
                    events.stablizer_sells.push(StablizerSell {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        sold: ev.sold.to_string(),
                        received: ev.received.to_string(),
                        tx_hash: tx_hash.clone(),
                        log_index: log.index() as u64,
                        block_num: block.number,
                        timestamp,
                    });
                    continue;
                }
                if let Some(ev) =
                    abi::stablizer::events::Buy::match_and_decode(log)
                {
                    events.stablizer_buys.push(StablizerBuy {
                        id: id.clone(),
                        user: fmt_addr(&ev.user),
                        purchased: ev.purchased.to_string(),
                        spent: ev.spent.to_string(),
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

    for e in events.factory_market_listeds {
        tables
            .create_row("factory_market_listed", &e.id)
            .set("c_token", &e.c_token)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.factory_action_pauseds {
        tables
            .create_row("factory_action_paused", &e.id)
            .set("c_token", &e.c_token)
            .set("action", &e.action)
            .set("pause_state", &e.pause_state)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.factory_distributed_borrower_comps {
        tables
            .create_row("factory_distributed_borrower_comp", &e.id)
            .set("c_token", &e.c_token)
            .set("borrower", &e.borrower)
            .set("comp_delta", &e.comp_delta)
            .set("comp_borrow_index", &e.comp_borrow_index)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.factory_distributed_supplier_comps {
        tables
            .create_row("factory_distributed_supplier_comp", &e.id)
            .set("c_token", &e.c_token)
            .set("supplier", &e.supplier)
            .set("comp_delta", &e.comp_delta)
            .set("comp_supply_index", &e.comp_supply_index)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.factory_new_collateral_factors {
        tables
            .create_row("factory_new_collateral_factor", &e.id)
            .set("c_token", &e.c_token)
            .set("old_collateral_factor_mantissa", &e.old_collateral_factor_mantissa)
            .set("new_collateral_factor_mantissa", &e.new_collateral_factor_mantissa)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.factory_new_close_factors {
        tables
            .create_row("factory_new_close_factor", &e.id)
            .set("old_close_factor_mantissa", &e.old_close_factor_mantissa)
            .set("new_close_factor_mantissa", &e.new_close_factor_mantissa)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.factory_new_liquidation_incentives {
        tables
            .create_row("factory_new_liquidation_incentive", &e.id)
            .set("old_liquidation_incentive_mantissa", &e.old_liquidation_incentive_mantissa)
            .set("new_liquidation_incentive_mantissa", &e.new_liquidation_incentive_mantissa)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.c_token_mints {
        tables
            .create_row("c_token_mint", &e.id)
            .set("pool", &e.pool)
            .set("minter", &e.minter)
            .set("mint_amount", &e.mint_amount)
            .set("mint_tokens", &e.mint_tokens)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.c_token_redeems {
        tables
            .create_row("c_token_redeem", &e.id)
            .set("pool", &e.pool)
            .set("redeemer", &e.redeemer)
            .set("redeem_amount", &e.redeem_amount)
            .set("redeem_tokens", &e.redeem_tokens)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.c_token_borrows {
        tables
            .create_row("c_token_borrow", &e.id)
            .set("pool", &e.pool)
            .set("borrower", &e.borrower)
            .set("borrow_amount", &e.borrow_amount)
            .set("account_borrows", &e.account_borrows)
            .set("total_borrows", &e.total_borrows)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.c_token_repay_borrows {
        tables
            .create_row("c_token_repay_borrow", &e.id)
            .set("pool", &e.pool)
            .set("payer", &e.payer)
            .set("borrower", &e.borrower)
            .set("repay_amount", &e.repay_amount)
            .set("account_borrows", &e.account_borrows)
            .set("total_borrows", &e.total_borrows)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.c_token_liquidate_borrows {
        tables
            .create_row("c_token_liquidate_borrow", &e.id)
            .set("pool", &e.pool)
            .set("liquidator", &e.liquidator)
            .set("borrower", &e.borrower)
            .set("repay_amount", &e.repay_amount)
            .set("c_token_collateral", &e.c_token_collateral)
            .set("seize_tokens", &e.seize_tokens)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.c_token_accrue_interests {
        tables
            .create_row("c_token_accrue_interest", &e.id)
            .set("pool", &e.pool)
            .set("cash_prior", &e.cash_prior)
            .set("interest_accumulated", &e.interest_accumulated)
            .set("borrow_index", &e.borrow_index)
            .set("total_borrows", &e.total_borrows)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.inv_owner_changeds {
        tables
            .create_row("inv_owner_changed", &e.id)
            .set("owner", &e.owner)
            .set("new_owner", &e.new_owner)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.dola_transfers {
        tables
            .create_row("dola_transfer", &e.id)
            .set("from", &e.from)
            .set("to", &e.to)
            .set("value", &e.value)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stablizer_sells {
        tables
            .create_row("stablizer_sell", &e.id)
            .set("user", &e.user)
            .set("sold", &e.sold)
            .set("received", &e.received)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    for e in events.stablizer_buys {
        tables
            .create_row("stablizer_buy", &e.id)
            .set("user", &e.user)
            .set("purchased", &e.purchased)
            .set("spent", &e.spent)
            .set("tx_hash", &e.tx_hash)
            .set("log_index", e.log_index as i64)
            .set("block_num", e.block_num as i64)
            .set("timestamp", e.timestamp as i64);
    }

    Ok(tables.to_database_changes())
}
