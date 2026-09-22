create table if not exists trades (
    id                text primary key,
    block_number       bigint not null,
    "timestamp"        bigint not null,
    tx_hash            text not null,
    "user"             text not null,
    counterparty       text not null,
    token_id           text not null,
    side               text not null, -- 'buy' | 'sell'
    token_amount       numeric not null,
    collateral_amount  numeric not null,
    price              text not null,
    exchange_version   int not null,
    exchange_address   text not null
);
create index if not exists trades_user_idx on trades ("user");
create index if not exists trades_token_id_idx on trades (token_id);
create index if not exists trades_block_number_idx on trades (block_number);

create table if not exists whale_alerts (
    id                text primary key,
    block_number      bigint not null,
    "timestamp"       bigint not null,
    tx_hash           text not null,
    "user"            text not null,
    token_id          text not null,
    collateral_amount numeric not null
);
create index if not exists whale_alerts_user_idx on whale_alerts ("user");

-- One row per (user, token_id): open position and cumulative cash flow.
-- Both columns are accumulated by the sink from `add` delta ops. Mark-to-
-- market PnL is not stored (it is nonlinear in token_amount, so it cannot be
-- accumulated) — use user_positions_pnl below. All amounts are raw USDC
-- atomic units (6 decimals).
create table if not exists user_positions (
    "user"          text not null,
    token_id        text not null,
    token_amount    numeric not null default 0,
    net_cash_flow   numeric not null default 0,
    primary key ("user", token_id)
);
create index if not exists user_positions_token_id_idx on user_positions (token_id);

-- Most recent fill price per outcome token, "N.NNNNNN" (collateral per token).
create table if not exists token_prices (
    token_id     text primary key,
    price        text not null,
    block_number bigint not null,
    "timestamp"  bigint not null
);

-- total_pnl = net_cash_flow + token_amount * latest_price (exact; equals
-- fully realized PnL once token_amount reaches 0 — see pnl.proto for why
-- this package does not report realized/unrealized as separate numbers).
-- Joining at query time re-marks every holder when a price moves, not just
-- the ones who traded since. Tokens with no fill yet are marked at cash flow.
create or replace view user_positions_pnl as
select
    p."user",
    p.token_id,
    p.token_amount,
    p.net_cash_flow,
    coalesce(tp.price, '') as latest_price,
    p.net_cash_flow + trunc(p.token_amount * coalesce(tp.price::numeric, 0)) as total_pnl
from user_positions p
left join token_prices tp on tp.token_id = p.token_id;

-- One row per user: lifetime traded notional and net cash flow across every
-- market, both accumulated by the sink. Total mark-to-market PnL per user is
-- sum(total_pnl) over user_positions_pnl.
create table if not exists user_pnl (
    "user"        text primary key,
    total_volume  numeric not null default 0,
    net_cash_flow numeric not null default 0
);

-- Metadata + resolution status only. Not joined to trades/markets volume in
-- this version: ConditionPreparation (where outcome_slot_count comes from)
-- doesn't carry the collateral token address that src/ctf_position.rs needs
-- to derive token_ids, and guessing it (v1 vs. v2-era "pUSD" collateral,
-- per the substreams-dev landing-page draft) is exactly the kind of
-- assumption this package avoids making without a way to verify it. Join
-- externally, from a PositionSplit/PositionsMerge event's own
-- collateral_token field, once needed.
create table if not exists markets (
    condition_id       text primary key,
    oracle             text not null default '',
    question_id        text not null default '',
    outcome_slot_count int not null default 0,
    resolved           boolean not null default false,
    payout_numerators  text not null default '', -- comma-separated
    created_block      bigint not null default 0,
    resolved_block     bigint not null default 0
);
