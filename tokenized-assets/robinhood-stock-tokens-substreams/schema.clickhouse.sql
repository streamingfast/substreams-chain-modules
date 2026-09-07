-- robinhood-stock-tokens ClickHouse schema. Inserts only - ReplacingMergeTree dedupes on
-- the ORDER BY key so a re-sunk block is idempotent.
--
-- Decimal columns: substreams-sink-sql (DatabaseChanges mode) cannot insert
-- into ClickHouse Decimal columns directly, so each exact value arrives as a
-- `<name>_str String` and `<name> Decimal(38,18)` is MATERIALIZED from it.
-- The sink skips MATERIALIZED columns on insert - query the Decimal column.

CREATE TABLE IF NOT EXISTS stock_swaps (
    tx_hash             String,
    log_index           UInt32,
    block_num           UInt64,
    block_ts            UInt64,
    block_time          DateTime,
    pool_id             String,
    ticker              LowCardinality(String),
    token               String,
    side                LowCardinality(String),
    shares_ui_str           String,
    shares_raw_adjusted_str String,
    quote_amount_str        String,
    amount_usd_str          String,
    price_usd_str           String,
    shares_ui           Decimal(38, 18) MATERIALIZED toDecimal128OrNull(shares_ui_str, 18),
    shares_raw_adjusted Decimal(38, 18) MATERIALIZED toDecimal128OrNull(shares_raw_adjusted_str, 18),
    quote_amount        Decimal(38, 18) MATERIALIZED toDecimal128OrNull(quote_amount_str, 18),
    amount_usd          Decimal(38, 18) MATERIALIZED toDecimal128OrNull(amount_usd_str, 18),
    price_usd           Decimal(38, 18) MATERIALIZED toDecimal128OrNull(price_usd_str, 18),
    quote_symbol        LowCardinality(String),
    quote_kind          LowCardinality(String),
    priced              Bool,
    sender              String,
    origin              String,
    fee                 UInt32,
    hook_address        String
) ENGINE = ReplacingMergeTree
ORDER BY (ticker, block_time, tx_hash, log_index);

CREATE TABLE IF NOT EXISTS chainlink_answers (
    tx_hash        String,
    log_index      UInt32,
    block_num      UInt64,
    block_ts       UInt64,
    block_time     DateTime,
    ticker         LowCardinality(String),
    feed           String,
    aggregator     String,
    answer_usd_str String,
    answer_usd     Decimal(38, 18) MATERIALIZED toDecimal128OrNull(answer_usd_str, 18),
    round_id       UInt64,
    updated_at     DateTime
) ENGINE = ReplacingMergeTree
ORDER BY (ticker, block_time, tx_hash, log_index);

CREATE TABLE IF NOT EXISTS basis_ticks (
    tx_hash         String,
    log_index       UInt32,
    block_num       UInt64,
    block_ts        UInt64,
    block_time      DateTime,
    ticker          LowCardinality(String),
    implied_usd_str String,
    ref_usd_str     String,
    amount_usd_str  String,
    implied_usd     Decimal(38, 18) MATERIALIZED toDecimal128OrNull(implied_usd_str, 18),
    ref_usd         Decimal(38, 18) MATERIALIZED toDecimal128OrNull(ref_usd_str, 18),
    amount_usd      Decimal(38, 18) MATERIALIZED toDecimal128OrNull(amount_usd_str, 18),
    ref_source      LowCardinality(String),
    ref_ts          DateTime,
    premium_bps     Int64,
    session         LowCardinality(String),
    side            LowCardinality(String)
) ENGINE = ReplacingMergeTree
ORDER BY (ticker, block_time, tx_hash, log_index);

CREATE TABLE IF NOT EXISTS stock_registry (
    ticker         LowCardinality(String),
    token          String,
    feed           String,
    aggregator     String,
    multiplier_str String,
    multiplier     Decimal(38, 18) MATERIALIZED toDecimal128OrNull(multiplier_str, 18),
    decimals       UInt32,
    status         LowCardinality(String),
    name           String,
    has_feed       Bool
) ENGINE = ReplacingMergeTree
ORDER BY token;

-- Hourly rollups. AggregatingMergeTree keeps partial states so avg/last are
-- exact across merges - query through the *_v views (or use -Merge yourself).
-- The SELECTs convert from the *_str columns so they do not depend on
-- MATERIALIZED columns being visible inside the insert trigger.

CREATE MATERIALIZED VIEW IF NOT EXISTS basis_hourly
ENGINE = AggregatingMergeTree
ORDER BY (ticker, hour)
AS SELECT
    ticker,
    toStartOfHour(block_time)                                                   AS hour,
    avgState(premium_bps)                                                       AS avg_premium_bps,
    minState(premium_bps)                                                       AS min_premium_bps,
    maxState(premium_bps)                                                       AS max_premium_bps,
    argMaxState(toDecimal128OrZero(implied_usd_str, 18), (block_num, log_index)) AS last_implied_usd,
    argMaxState(toDecimal128OrZero(ref_usd_str, 18), (block_num, log_index))     AS last_ref_usd,
    countState()                                                                AS swap_count,
    sumState(toDecimal128OrZero(amount_usd_str, 18))                            AS volume_usd
FROM basis_ticks
GROUP BY ticker, hour;

CREATE VIEW IF NOT EXISTS basis_hourly_v AS
SELECT
    ticker,
    hour,
    avgMerge(avg_premium_bps)      AS avg_premium_bps,
    minMerge(min_premium_bps)      AS min_premium_bps,
    maxMerge(max_premium_bps)      AS max_premium_bps,
    argMaxMerge(last_implied_usd)  AS last_implied_usd,
    argMaxMerge(last_ref_usd)      AS last_ref_usd,
    countMerge(swap_count)         AS swap_count,
    sumMerge(volume_usd)           AS volume_usd
FROM basis_hourly
GROUP BY ticker, hour;

CREATE MATERIALIZED VIEW IF NOT EXISTS swaps_hourly
ENGINE = SummingMergeTree
ORDER BY (ticker, hour)
AS SELECT
    ticker,
    toStartOfHour(block_time)                 AS hour,
    countIf(side = 'buy')                     AS buys,
    countIf(side = 'sell')                    AS sells,
    sum(toDecimal128OrZero(amount_usd_str, 18)) AS volume_usd,
    sum(toDecimal128OrZero(shares_ui_str, 18))  AS shares
FROM stock_swaps
GROUP BY ticker, hour;

CREATE VIEW IF NOT EXISTS swaps_hourly_v AS
SELECT
    ticker,
    hour,
    sum(buys)        AS buys,
    sum(sells)       AS sells,
    sum(volume_usd)  AS volume_usd,
    sum(shares)      AS shares
FROM swaps_hourly
GROUP BY ticker, hour;
