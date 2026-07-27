-- Superfluid ClickHouse projections — CFA internal (L0–L2)
-- Apply first. API surface: sql/hasura/api_views_*.sql

CREATE OR REPLACE VIEW v_flow_updated_live AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    lower(token)    AS token,
    lower(sender)   AS sender,
    lower(receiver) AS receiver,
    flow_rate,
    toInt256OrZero(flow_rate) AS flow_rate_i,
    total_sender_flow_rate,
    total_receiver_flow_rate,
    user_data
FROM flow_updated_events
WHERE _deleted_ = false;

CREATE OR REPLACE VIEW v_flow_updated_extension_live AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    lower(flow_operator) AS flow_operator,
    deposit,
    toInt256OrZero(deposit) AS deposit_i
FROM flow_updated_extension_events
WHERE _deleted_ = false;

-- ---------------------------------------------------------------------------
-- L1: pair key + deposit from extension (same tx)
-- ---------------------------------------------------------------------------
-- Multi-flow txs can emit FU1, FU2, FUE1, FUE2 (or interleaved pairs). ASOF on
-- log_index alone mis-pairs the second FlowUpdated with the first Extension.
-- Pair by ordinal within (tx, block): nth FlowUpdated ↔ nth Extension.
CREATE OR REPLACE VIEW v_flow_updates_base AS
SELECT
    f.id,
    f.block_number,
    f.block_timestamp,
    f.transaction_hash,
    f.log_index,
    f.token,
    f.sender,
    f.receiver,
    f.flow_rate,
    f.flow_rate_i,
    f.user_data,
    concat(f.sender, '-', f.receiver, '-', f.token) AS stream_pair_key,
    e.deposit,
    e.deposit_i,
    e.flow_operator
FROM
(
    SELECT
        *,
        row_number() OVER (
            PARTITION BY transaction_hash, block_number
            ORDER BY log_index ASC
        ) AS flow_ord
    FROM v_flow_updated_live
) AS f
LEFT JOIN
(
    SELECT
        *,
        row_number() OVER (
            PARTITION BY transaction_hash, block_number
            ORDER BY log_index ASC
        ) AS ext_ord
    FROM v_flow_updated_extension_live
) AS e
    ON f.transaction_hash = e.transaction_hash
   AND f.block_number = e.block_number
   AND f.flow_ord = e.ext_ord;

-- ---------------------------------------------------------------------------
-- L2: event classification + stream revision
-- create  = prior rate 0 (or first), new rate != 0
-- update  = prior != 0 and new != 0
-- terminate = prior != 0 and new == 0
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW v_flow_updates_enriched AS
SELECT
    id,
    block_number,
    block_timestamp,
    transaction_hash,
    log_index,
    token,
    sender,
    receiver,
    stream_pair_key,
    flow_rate,
    flow_rate_i,
    user_data,
    deposit,
    deposit_i,
    flow_operator,
    prev_flow_rate_i,
    multiIf(
        (prev_flow_rate_i = 0 OR isNull(prev_flow_rate_i)) AND flow_rate_i != 0, 'create',
        prev_flow_rate_i != 0 AND flow_rate_i != 0, 'update',
        prev_flow_rate_i != 0 AND flow_rate_i = 0, 'terminate',
        'noop'
    ) AS event_type,
    sumIf(1, (prev_flow_rate_i = 0 OR isNull(prev_flow_rate_i)) AND flow_rate_i != 0)
        OVER (
            PARTITION BY stream_pair_key
            ORDER BY block_number ASC, log_index ASC
            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
        ) AS revision,
    concat(
        stream_pair_key,
        '-',
        toString(
            sumIf(1, (prev_flow_rate_i = 0 OR isNull(prev_flow_rate_i)) AND flow_rate_i != 0)
                OVER (
                    PARTITION BY stream_pair_key
                    ORDER BY block_number ASC, log_index ASC
                    ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                )
        )
    ) AS stream_id
FROM
(
    SELECT
        *,
        lagInFrame(flow_rate_i, 1, toInt256(0)) OVER (
            PARTITION BY stream_pair_key
            ORDER BY block_number ASC, log_index ASC
            ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
        ) AS prev_flow_rate_i
    FROM v_flow_updates_base
);

-- ---------------------------------------------------------------------------
-- L2: stream periods (constant rate between consecutive events on a stream)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW v_stream_periods AS
SELECT
    concat(stream_id, '-', toString(block_number), '-', toString(log_index)) AS period_id,
    stream_id,
    stream_pair_key,
    token,
    sender,
    receiver,
    flow_rate_i AS flow_rate,
    deposit_i AS deposit,
    block_timestamp AS started_at,
    block_number AS started_at_block,
    transaction_hash AS started_tx,
    log_index AS started_log_index,
    next_block_timestamp AS stopped_at,
    next_block_number AS stopped_at_block,
    if(
        isNull(next_block_timestamp) OR flow_rate_i = 0,
        toInt256(0),
        flow_rate_i * toInt256(next_block_timestamp - block_timestamp)
    ) AS amount_streamed_in_period,
    event_type AS opened_by_event_type,
    if(flow_rate_i = 0, 1, 0) AS is_terminal_marker
FROM
(
    SELECT
        *,
        leadInFrame(toNullable(block_timestamp), 1, CAST(NULL AS Nullable(UInt64))) OVER (
            PARTITION BY stream_id
            ORDER BY block_number ASC, log_index ASC
            ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
        ) AS next_block_timestamp,
        leadInFrame(toNullable(block_number), 1, CAST(NULL AS Nullable(UInt64))) OVER (
            PARTITION BY stream_id
            ORDER BY block_number ASC, log_index ASC
            ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
        ) AS next_block_number
    FROM v_flow_updates_enriched
    WHERE event_type != 'noop'
)
WHERE flow_rate_i != 0
   OR event_type = 'terminate';

-- Periods that actually stream (exclude pure terminate rows as open periods)
CREATE OR REPLACE VIEW v_stream_periods_active AS
SELECT *
FROM v_stream_periods
WHERE flow_rate != 0;

-- ---------------------------------------------------------------------------
-- L2: current stream state (latest event per stream_id)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW v_stream_streamed_closed AS
SELECT
    stream_id,
    sum(amount_streamed_in_period) AS streamed_closed_periods
FROM v_stream_periods_active
WHERE stopped_at IS NOT NULL
GROUP BY stream_id;

CREATE OR REPLACE VIEW v_streams_current AS
SELECT
    s.stream_id,
    s.stream_pair_key,
    s.token,
    s.sender,
    s.receiver,
    s.current_flow_rate,
    s.deposit,
    s.user_data,
    s.opened_at,
    s.opened_at_block,
    s.updated_at,
    s.updated_at_block,
    s.closed_at,
    s.is_open,
    ifNull(c.streamed_closed_periods, toInt256(0)) AS streamed_until_updated_at
FROM
(
    SELECT
        stream_id,
        any(stream_pair_key) AS stream_pair_key,
        any(token) AS token,
        any(sender) AS sender,
        any(receiver) AS receiver,
        argMax(flow_rate_i, (block_number, log_index)) AS current_flow_rate,
        argMax(deposit_i, (block_number, log_index)) AS deposit,
        argMax(user_data, (block_number, log_index)) AS user_data,
        minIf(block_timestamp, event_type = 'create') AS opened_at,
        minIf(block_number, event_type = 'create') AS opened_at_block,
        max(block_timestamp) AS updated_at,
        max(block_number) AS updated_at_block,
        if(
            argMax(flow_rate_i, (block_number, log_index)) = 0,
            max(block_timestamp),
            CAST(NULL AS Nullable(UInt64))
        ) AS closed_at,
        if(argMax(flow_rate_i, (block_number, log_index)) = 0, 0, 1) AS is_open
    FROM v_flow_updates_enriched
    WHERE event_type != 'noop'
      AND revision > 0
    GROUP BY stream_id
) AS s
LEFT JOIN v_stream_streamed_closed AS c ON c.stream_id = s.stream_id;

-- ---------------------------------------------------------------------------
-- L3 / L4: account-facing CFA APIs
-- ---------------------------------------------------------------------------
-- Deterministic streamed amount at an arbitrary unix timestamp (parity / as-of-H).
-- Usage: SELECT streamed_amount_as_of(streamed_until_updated_at, current_flow_rate,
--          updated_at, toUInt64(<H_ts>), is_open) FROM v_streams_current ...
CREATE OR REPLACE FUNCTION streamed_amount_as_of AS (
    streamed_until,
    flow_rate,
    updated_at,
    as_of_ts,
    is_open
) -> if(
    is_open = 1 AND as_of_ts >= updated_at,
    streamed_until + flow_rate * toInt256(toInt64(as_of_ts) - toInt64(updated_at)),
    streamed_until
);
