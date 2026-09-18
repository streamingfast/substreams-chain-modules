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

-- One row per (user, token_id): open position and mark-to-market PnL.
-- total_pnl = net_cash_flow + token_amount * latest_price (exact; equals
-- fully realized PnL once token_amount reaches 0 — see pnl.proto for why
-- this package does not report realized/unrealized as separate numbers).
-- All amounts are raw USDC atomic units (6 decimals).
create table if not exists user_positions (
    "user"          text not null,
    token_id        text not null,
    token_amount    numeric not null default 0,
    net_cash_flow   numeric not null default 0,
    latest_price    text not null default '',
    total_pnl       numeric not null default 0,
    primary key ("user", token_id)
);

-- One row per user: totals across every market traded. total_pnl sums
-- user_positions.total_pnl-equivalent (cash_flow + open positions marked at
-- latest price) across all of the user's tokens.
create table if not exists user_pnl (
    "user"        text primary key,
    total_volume  numeric not null default 0,
    net_cash_flow numeric not null default 0
);

-- Metadata + resolution status only. Not joined to trades/markets volume in
-- this version: linking a condition_id to its token_ids requires Gnosis
-- CTF's elliptic-curve collection-ID derivation, which this package does
-- not implement (see pnl.proto scope note). Join externally once available.
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
