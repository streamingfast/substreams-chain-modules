-- Uniswap v4 ClickHouse HOL (Tier B) — Base
-- Apply after from-proto event tables exist:
--   ./scripts/apply_views.sh

-- ---------------------------------------------------------------------------
-- L0 live filters
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW v_initialize_live AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    lower(pool_id) AS pool_id,
    lower(currency0) AS currency0,
    lower(currency1) AS currency1,
    fee,
    tick_spacing,
    lower(hooks) AS hooks,
    sqrt_price_x96,
    tick
FROM initialize_events
WHERE _deleted_ = false;

CREATE OR REPLACE VIEW v_swap_live AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    lower(pool_id) AS pool_id,
    lower(sender) AS sender,
    amount0,
    amount1,
    sqrt_price_x96,
    liquidity,
    tick,
    fee
FROM swap_events
WHERE _deleted_ = false;

CREATE OR REPLACE VIEW v_modify_liquidity_live AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    lower(pool_id) AS pool_id,
    lower(sender) AS sender,
    tick_lower,
    tick_upper,
    liquidity_delta,
    salt
FROM modify_liquidity_events
WHERE _deleted_ = false;

CREATE OR REPLACE VIEW v_position_transfer_live AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    lower(from_address) AS from_address,
    lower(to_address) AS to_address,
    token_id
FROM position_transfer_events
WHERE _deleted_ = false;

-- ---------------------------------------------------------------------------
-- L4 API: pools (create + latest state from swaps / liquidity)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW api_pools AS
SELECT
    i.pool_id AS pool_id,
    i.currency0 AS token0,
    i.currency1 AS token1,
    i.fee AS fee,
    i.tick_spacing AS tick_spacing,
    i.hooks AS hooks,
    i.block_timestamp AS created_at,
    i.block_number AS created_at_block,
    ifNull(s.sqrt_price_x96, i.sqrt_price_x96) AS sqrt_price_x96,
    ifNull(s.tick, i.tick) AS tick,
    ifNull(s.liquidity, '0') AS liquidity,
    ifNull(s.updated_at, i.block_timestamp) AS updated_at,
    ifNull(s.updated_at_block, i.block_number) AS updated_at_block,
    ifNull(sc.swap_count, toUInt64(0)) AS swap_count
FROM
(
    -- first Initialize per pool (pool_id unique)
    SELECT
        pool_id,
        argMin(currency0, (block_number, log_index)) AS currency0,
        argMin(currency1, (block_number, log_index)) AS currency1,
        argMin(fee, (block_number, log_index)) AS fee,
        argMin(tick_spacing, (block_number, log_index)) AS tick_spacing,
        argMin(hooks, (block_number, log_index)) AS hooks,
        argMin(sqrt_price_x96, (block_number, log_index)) AS sqrt_price_x96,
        argMin(tick, (block_number, log_index)) AS tick,
        min(block_timestamp) AS block_timestamp,
        min(block_number) AS block_number
    FROM v_initialize_live
    GROUP BY pool_id
) AS i
LEFT JOIN
(
    -- latest swap carries post-swap sqrtPrice, liquidity, tick
    SELECT
        pool_id,
        argMax(sqrt_price_x96, (block_number, log_index)) AS sqrt_price_x96,
        argMax(tick, (block_number, log_index)) AS tick,
        argMax(liquidity, (block_number, log_index)) AS liquidity,
        max(block_timestamp) AS updated_at,
        max(block_number) AS updated_at_block
    FROM v_swap_live
    GROUP BY pool_id
) AS s ON i.pool_id = s.pool_id
LEFT JOIN
(
    SELECT pool_id, count() AS swap_count
    FROM v_swap_live
    GROUP BY pool_id
) AS sc ON i.pool_id = sc.pool_id;

-- ---------------------------------------------------------------------------
-- L4 API: swaps / modify liquidity (event analytics)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW api_swaps AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    pool_id,
    sender,
    amount0,
    amount1,
    sqrt_price_x96,
    liquidity,
    tick,
    fee
FROM v_swap_live;

CREATE OR REPLACE VIEW api_modify_liquidity AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    pool_id,
    sender,
    tick_lower,
    tick_upper,
    liquidity_delta,
    salt
FROM v_modify_liquidity_live;

-- ---------------------------------------------------------------------------
-- L4 API: positions (latest owner from NFT transfers)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW api_positions AS
SELECT
    token_id AS token_id,
    argMax(to_address, (block_number, log_index)) AS owner,
    argMin(to_address, (block_number, log_index)) AS origin_or_first_to,
    min(block_timestamp) AS first_transfer_at,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM v_position_transfer_live
GROUP BY token_id;

-- ---------------------------------------------------------------------------
-- L4 API: tokens seen as pool currencies (no eth_call metadata in core)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW api_tokens AS
SELECT
    token AS token,
    min(first_seen) AS first_seen_block,
    min(first_seen_ts) AS first_seen
FROM
(
    SELECT currency0 AS token, min(block_number) AS first_seen, min(block_timestamp) AS first_seen_ts
    FROM v_initialize_live
    GROUP BY currency0
    UNION ALL
    SELECT currency1 AS token, min(block_number) AS first_seen, min(block_timestamp) AS first_seen_ts
    FROM v_initialize_live
    GROUP BY currency1
)
GROUP BY token;
