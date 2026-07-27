-- Superfluid ClickHouse projections — IDA/GDA internal (v_*)
-- Apply after 01. API surface: sql/hasura/api_views_*.sql

CREATE OR REPLACE VIEW v_ida_index_updates AS
SELECT
    lower(publisher) AS publisher,
    lower(token) AS token,
    index_id,
    block_number,
    log_index,
    block_timestamp,
    toInt256OrZero(old_index_value) AS old_index_value_i,
    toInt256OrZero(new_index_value) AS new_index_value_i,
    toInt256OrZero(total_units_pending) AS total_units_pending_i,
    toInt256OrZero(total_units_approved) AS total_units_approved_i,
    toInt256OrZero(total_units_pending) + toInt256OrZero(total_units_approved) AS total_units_i,
    (toInt256OrZero(new_index_value) - toInt256OrZero(old_index_value))
        * (toInt256OrZero(total_units_pending) + toInt256OrZero(total_units_approved))
        AS distribution_delta_i
FROM index_updated_events
WHERE _deleted_ = false;

CREATE OR REPLACE VIEW v_ida_subscription_keys AS
SELECT DISTINCT
    lower(subscriber) AS subscriber,
    lower(publisher) AS publisher,
    lower(token) AS token,
    index_id
FROM
(
    SELECT subscriber, publisher, token, index_id FROM subscription_units_updated_events WHERE _deleted_ = false
    UNION ALL
    SELECT subscriber, publisher, token, index_id FROM subscription_approved_events WHERE _deleted_ = false
    UNION ALL
    SELECT subscriber, publisher, token, index_id FROM subscription_revoked_events WHERE _deleted_ = false
    UNION ALL
    SELECT subscriber, publisher, token, index_id FROM subscription_distribution_claimed_events WHERE _deleted_ = false
    UNION ALL
    SELECT subscriber, publisher, token, index_id FROM index_subscribed_events WHERE _deleted_ = false
    UNION ALL
    SELECT subscriber, publisher, token, index_id FROM index_unsubscribed_events WHERE _deleted_ = false
);

-- Latest units per subscription
CREATE OR REPLACE VIEW v_ida_subscription_units AS
SELECT
    lower(subscriber) AS subscriber,
    lower(publisher) AS publisher,
    lower(token) AS token,
    index_id,
    argMax(toInt256OrZero(units), (block_number, log_index)) AS units,
    max(block_timestamp) AS units_updated_at,
    max(block_number) AS units_updated_at_block
FROM subscription_units_updated_events
WHERE _deleted_ = false
GROUP BY subscriber, publisher, token, index_id;

-- Approval lifecycle: Approve=1, Revoke=0 (argMax by order)
CREATE OR REPLACE VIEW v_ida_subscription_approvals AS
SELECT
    lower(subscriber) AS subscriber,
    lower(publisher) AS publisher,
    lower(token) AS token,
    index_id,
    argMax(approved, (block_number, log_index)) AS approved,
    max(block_timestamp) AS approval_updated_at,
    max(block_number) AS approval_updated_at_block
FROM
(
    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        1 AS approved
    FROM subscription_approved_events
    WHERE _deleted_ = false

    UNION ALL

    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        0 AS approved
    FROM subscription_revoked_events
    WHERE _deleted_ = false
)
GROUP BY subscriber, publisher, token, index_id;

-- IndexSubscribed / IndexUnsubscribed → subscribed flag (P2 parity gap fix)
CREATE OR REPLACE VIEW v_ida_subscription_subscribed AS
SELECT
    lower(subscriber) AS subscriber,
    lower(publisher) AS publisher,
    lower(token) AS token,
    index_id,
    argMax(subscribed, (block_number, log_index)) AS subscribed,
    max(block_timestamp) AS subscribed_updated_at
FROM
(
    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        1 AS subscribed
    FROM index_subscribed_events
    WHERE _deleted_ = false

    UNION ALL

    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        0 AS subscribed
    FROM index_unsubscribed_events
    WHERE _deleted_ = false
)
GROUP BY subscriber, publisher, token, index_id;

-- Index value timeline for ASOF (piecewise-constant after each IndexUpdated)
CREATE OR REPLACE VIEW v_ida_index_value_timeline AS
SELECT
    publisher,
    token,
    index_id,
    block_number,
    log_index,
    block_timestamp,
    new_index_value_i AS index_value
FROM v_ida_index_updates;

-- Settlement stream: events that advance totalAmountReceivedUntilUpdatedAt
-- (approve / revoke / claim / units_updated — same set as subgraph handlers).
-- balanceDelta = units_before * (indexValue_now - indexValueUntil_before)
CREATE OR REPLACE VIEW v_ida_subscription_settlements_raw AS
SELECT
    lower(subscriber) AS subscriber,
    lower(publisher) AS publisher,
    lower(token) AS token,
    index_id,
    block_number,
    log_index,
    block_timestamp,
    kind,
    units_after_i
FROM
(
    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        'units_updated' AS kind,
        toInt256OrZero(units) AS units_after_i
    FROM subscription_units_updated_events
    WHERE _deleted_ = false

    UNION ALL

    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        'approved' AS kind,
        CAST(NULL AS Nullable(Int256)) AS units_after_i
    FROM subscription_approved_events
    WHERE _deleted_ = false

    UNION ALL

    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        'revoked' AS kind,
        CAST(NULL AS Nullable(Int256)) AS units_after_i
    FROM subscription_revoked_events
    WHERE _deleted_ = false

    UNION ALL

    SELECT
        subscriber, publisher, token, index_id,
        block_number, log_index, block_timestamp,
        'claimed' AS kind,
        CAST(NULL AS Nullable(Int256)) AS units_after_i
    FROM subscription_distribution_claimed_events
    WHERE _deleted_ = false
);

-- Carry units forward; units_for_delta = prior filled units (pre-mutation)
CREATE OR REPLACE VIEW v_ida_subscription_settlements AS
SELECT
    subscriber,
    publisher,
    token,
    index_id,
    block_number,
    log_index,
    block_timestamp,
    kind,
    units_after_i,
    lagInFrame(units_filled_i, 1, toInt256(0)) OVER (
        PARTITION BY subscriber, publisher, token, index_id
        ORDER BY block_number ASC, log_index ASC
        ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
    ) AS units_for_delta_i
FROM
(
    SELECT
        subscriber,
        publisher,
        token,
        index_id,
        block_number,
        log_index,
        block_timestamp,
        kind,
        units_after_i,
        ifNull(
            last_value(units_after_i) IGNORE NULLS OVER (
                PARTITION BY subscriber, publisher, token, index_id
                ORDER BY block_number ASC, log_index ASC
                ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
            ),
            toInt256(0)
        ) AS units_filled_i
    FROM v_ida_subscription_settlements_raw
);

-- Attach index value at settle time + prior indexValueUntil; amount delta
CREATE OR REPLACE VIEW v_ida_subscription_settlements_enriched AS
SELECT
    st.subscriber,
    st.publisher,
    st.token,
    st.index_id,
    st.block_number,
    st.log_index,
    st.block_timestamp,
    st.kind,
    st.units_for_delta_i,
    idx_val AS index_value_at_event,
    lagInFrame(idx_val, 1, toInt256(0)) OVER (
        PARTITION BY st.subscriber, st.publisher, st.token, st.index_id
        ORDER BY st.block_number ASC, st.log_index ASC
        ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
    ) AS index_value_until_before,
    st.units_for_delta_i * (
        idx_val
        - lagInFrame(idx_val, 1, toInt256(0)) OVER (
            PARTITION BY st.subscriber, st.publisher, st.token, st.index_id
            ORDER BY st.block_number ASC, st.log_index ASC
            ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
        )
    ) AS amount_delta_i
FROM
(
    SELECT
        st.*,
        ifNull(idx.index_value, toInt256(0)) AS idx_val
    FROM v_ida_subscription_settlements AS st
    ASOF LEFT JOIN v_ida_index_value_timeline AS idx
        ON st.publisher = idx.publisher
       AND st.token = idx.token
       AND st.index_id = idx.index_id
       AND st.block_number >= idx.block_number
) AS st;

CREATE OR REPLACE VIEW v_ida_subscription_amount_received AS
SELECT
    subscriber,
    publisher,
    token,
    index_id,
    sum(amount_delta_i) AS total_amount_received_until_updated_at,
    argMax(index_value_at_event, (block_number, log_index)) AS index_value_until_updated_at,
    max(block_timestamp) AS last_settled_at,
    max(block_number) AS last_settled_at_block
FROM v_ida_subscription_settlements_enriched
GROUP BY subscriber, publisher, token, index_id;

-- Latest index value only (avoid joining complex api_indexes in this view)
CREATE OR REPLACE VIEW v_ida_index_value_latest AS
SELECT
    publisher,
    token,
    index_id,
    argMax(new_index_value_i, (block_number, log_index)) AS index_value
FROM v_ida_index_updates
GROUP BY publisher, token, index_id;

-- Unified IndexSubscription HOL (subgraph IndexSubscription)
CREATE OR REPLACE VIEW v_gda_pool_flow_latest AS
SELECT
    lower(pool) AS pool,
    lower(token) AS token,
    argMax(toInt256OrZero(new_total_distribution_flow_rate), (block_number, log_index)) AS flow_rate,
    argMax(toInt256OrZero(adjustment_flow_rate), (block_number, log_index)) AS adjustment_flow_rate,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM flow_distribution_updated_events
WHERE _deleted_ = false
GROUP BY pool, token;

-- Instant distribution totals per pool
CREATE OR REPLACE VIEW v_gda_pool_instant AS
SELECT
    lower(pool) AS pool,
    lower(token) AS token,
    sum(toInt256OrZero(actual_amount)) AS total_amount_instantly_distributed_until_updated_at,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM instant_distribution_updated_events
WHERE _deleted_ = false
GROUP BY pool, token;

-- Buffer: prefer latest absolute total_buffer_amount when present; else sum of deltas
CREATE OR REPLACE VIEW v_gda_pool_buffer AS
SELECT
    lower(pool) AS pool,
    lower(token) AS token,
    argMax(toInt256OrZero(total_buffer_amount), (block_number, log_index)) AS total_buffer,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM buffer_adjusted_events
WHERE _deleted_ = false
GROUP BY pool, token;

-- Flowed distribution: periods of constant pool flow_rate between FlowDistributionUpdated events
CREATE OR REPLACE VIEW v_gda_pool_flow_periods AS
SELECT
    pool,
    token,
    flow_rate_after AS flow_rate,
    block_timestamp AS started_at,
    block_number AS started_at_block,
    next_ts AS stopped_at,
    if(
        isNull(next_ts) OR flow_rate_after = 0,
        toInt256(0),
        flow_rate_after * toInt256(toInt64(next_ts) - toInt64(block_timestamp))
    ) AS amount_flowed_in_period
FROM
(
    SELECT
        lower(pool) AS pool,
        lower(token) AS token,
        block_number,
        log_index,
        block_timestamp,
        toInt256OrZero(new_total_distribution_flow_rate) AS flow_rate_after,
        leadInFrame(toNullable(block_timestamp), 1, CAST(NULL AS Nullable(UInt64))) OVER (
            PARTITION BY lower(pool), lower(token)
            ORDER BY block_number ASC, log_index ASC
            ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
        ) AS next_ts
    FROM flow_distribution_updated_events
    WHERE _deleted_ = false
);

CREATE OR REPLACE VIEW v_gda_pool_flowed AS
SELECT
    pool,
    token,
    sum(amount_flowed_in_period) AS total_amount_flowed_distributed_until_updated_at
FROM v_gda_pool_flow_periods
WHERE stopped_at IS NOT NULL
GROUP BY pool, token;

-- Member units aggregate → pool total_units + connected/disconnected split
CREATE OR REPLACE VIEW v_gda_member_units_latest AS
SELECT
    lower(contract) AS pool,
    lower(member) AS member,
    lower(token) AS token,
    argMax(toInt256OrZero(new_units), (block_number, log_index)) AS units,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM member_units_updated_events
WHERE _deleted_ = false
GROUP BY pool, member, token;

CREATE OR REPLACE VIEW v_gda_member_connected_latest AS
SELECT
    lower(pool) AS pool,
    lower(account) AS member,
    lower(token) AS token,
    argMax(connected, (block_number, log_index)) AS connected,
    max(block_timestamp) AS updated_at
FROM pool_connection_updated_events
WHERE _deleted_ = false
GROUP BY pool, member, token;

CREATE OR REPLACE VIEW v_gda_pool_unit_stats AS
SELECT
    m.pool,
    m.token,
    sum(m.units) AS total_units,
    sumIf(m.units, ifNull(c.connected, 0) = 1) AS total_connected_units,
    sumIf(m.units, ifNull(c.connected, 0) = 0) AS total_disconnected_units,
    countIf(m.units > 0) AS total_members,
    countIf(m.units > 0 AND ifNull(c.connected, 0) = 1) AS total_connected_members,
    countIf(m.units > 0 AND ifNull(c.connected, 0) = 0) AS total_disconnected_members
FROM v_gda_member_units_latest AS m
LEFT JOIN v_gda_member_connected_latest AS c
    ON m.pool = c.pool AND m.member = c.member AND m.token = c.token
GROUP BY m.pool, m.token;

-- Pool HOL (subgraph Pool)
CREATE OR REPLACE VIEW v_gda_distributor_flow_latest AS
SELECT
    lower(pool) AS pool,
    lower(token) AS token,
    lower(distributor) AS distributor,
    argMax(toInt256OrZero(new_distributor_to_pool_flow_rate), (block_number, log_index)) AS flow_rate,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM flow_distribution_updated_events
WHERE _deleted_ = false
GROUP BY pool, token, distributor;

CREATE OR REPLACE VIEW v_gda_distributor_instant AS
SELECT
    lower(pool) AS pool,
    lower(token) AS token,
    lower(distributor) AS distributor,
    sum(toInt256OrZero(actual_amount)) AS total_amount_instantly_distributed_until_updated_at,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM instant_distribution_updated_events
WHERE _deleted_ = false
GROUP BY pool, token, distributor;

CREATE OR REPLACE VIEW v_gda_distributor_buffer AS
SELECT
    lower(pool) AS pool,
    lower(token) AS token,
    lower(from) AS distributor,
    argMax(toInt256OrZero(new_buffer_amount), (block_number, log_index)) AS total_buffer,
    max(block_timestamp) AS updated_at,
    max(block_number) AS updated_at_block
FROM buffer_adjusted_events
WHERE _deleted_ = false
GROUP BY pool, token, distributor;

CREATE OR REPLACE VIEW v_gda_distributor_flow_periods AS
SELECT
    pool,
    token,
    distributor,
    flow_rate_after AS flow_rate,
    block_timestamp AS started_at,
    next_ts AS stopped_at,
    if(
        isNull(next_ts) OR flow_rate_after = 0,
        toInt256(0),
        flow_rate_after * toInt256(toInt64(next_ts) - toInt64(block_timestamp))
    ) AS amount_flowed_in_period
FROM
(
    SELECT
        lower(pool) AS pool,
        lower(token) AS token,
        lower(distributor) AS distributor,
        block_number,
        log_index,
        block_timestamp,
        toInt256OrZero(new_distributor_to_pool_flow_rate) AS flow_rate_after,
        leadInFrame(toNullable(block_timestamp), 1, CAST(NULL AS Nullable(UInt64))) OVER (
            PARTITION BY lower(pool), lower(token), lower(distributor)
            ORDER BY block_number ASC, log_index ASC
            ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
        ) AS next_ts
    FROM flow_distribution_updated_events
    WHERE _deleted_ = false
);

CREATE OR REPLACE VIEW v_gda_distributor_flowed AS
SELECT
    pool,
    token,
    distributor,
    sum(amount_flowed_in_period) AS total_amount_flowed_distributed_until_updated_at
FROM v_gda_distributor_flow_periods
WHERE stopped_at IS NOT NULL
GROUP BY pool, token, distributor;

CREATE OR REPLACE VIEW v_gda_distributor_keys AS
SELECT pool, token, distributor FROM v_gda_distributor_flow_latest
UNION DISTINCT
SELECT pool, token, distributor FROM v_gda_distributor_instant
UNION DISTINCT
SELECT pool, token, distributor FROM v_gda_distributor_buffer;

CREATE OR REPLACE VIEW v_ida_indexes_created AS
SELECT
    lower(publisher) AS publisher,
    lower(token) AS token,
    index_id,
    concat(lower(publisher), '-', lower(token), '-', index_id) AS index_entity_id,
    block_timestamp AS created_at,
    block_number AS created_at_block,
    user_data
FROM index_created_events
WHERE _deleted_ = false;

-- Per IndexUpdated row: distribution delta = (new - old) * (pending + approved units)
