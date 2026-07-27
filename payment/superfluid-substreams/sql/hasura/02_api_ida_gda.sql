-- Hasura GraphQL API surface — tokens, IDA, GDA (api_*)
-- Depends on sql/clickhouse/02_views_ida_gda_tokens.sql
-- Applied by scripts/apply_views.sh after clickhouse views.

CREATE OR REPLACE VIEW api_tokens AS
SELECT
    token,
    min(created_at) AS created_at,
    min(created_at_block) AS created_at_block,
    any(source) AS source
FROM
(
    SELECT
        lower(token) AS token,
        block_timestamp AS created_at,
        block_number AS created_at_block,
        'SuperTokenCreated' AS source
    FROM super_token_created_events
    WHERE _deleted_ = false

    UNION ALL

    SELECT
        lower(token) AS token,
        block_timestamp AS created_at,
        block_number AS created_at_block,
        'CustomSuperTokenCreated' AS source
    FROM custom_super_token_created_events
    WHERE _deleted_ = false
)
GROUP BY token;

-- ---------------------------------------------------------------------------
-- IDA: indexes (created + latest update state + distributed totals)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW api_indexes AS
SELECT
    c.publisher AS publisher,
    c.token AS token,
    c.index_id AS index_id,
    c.index_entity_id AS index_entity_id,
    c.created_at AS created_at,
    c.created_at_block AS created_at_block,
    ifNull(u.index_value, toInt256(0)) AS index_value,
    ifNull(u.total_units_pending, toInt256(0)) AS total_units_pending,
    ifNull(u.total_units_approved, toInt256(0)) AS total_units_approved,
    ifNull(u.total_units_pending, toInt256(0))
        + ifNull(u.total_units_approved, toInt256(0)) AS total_units,
    ifNull(d.total_amount_distributed_until_updated_at, toInt256(0))
        AS total_amount_distributed_until_updated_at,
    ifNull(u.updated_at, c.created_at) AS updated_at,
    ifNull(u.updated_at_block, c.created_at_block) AS updated_at_block
FROM v_ida_indexes_created AS c
LEFT JOIN
(
    SELECT
        publisher,
        token,
        index_id,
        argMax(new_index_value_i, (block_number, log_index)) AS index_value,
        argMax(total_units_pending_i, (block_number, log_index)) AS total_units_pending,
        argMax(total_units_approved_i, (block_number, log_index)) AS total_units_approved,
        max(block_timestamp) AS updated_at,
        max(block_number) AS updated_at_block
    FROM v_ida_index_updates
    GROUP BY publisher, token, index_id
) AS u
    ON c.publisher = u.publisher
   AND c.token = u.token
   AND c.index_id = u.index_id
LEFT JOIN
(
    SELECT
        publisher,
        token,
        index_id,
        sum(distribution_delta_i) AS total_amount_distributed_until_updated_at
    FROM v_ida_index_updates
    GROUP BY publisher, token, index_id
) AS d
    ON c.publisher = d.publisher
   AND c.token = d.token
   AND c.index_id = d.index_id;

-- ---------------------------------------------------------------------------
-- IDA: subscription HOL — full lifecycle + amount-received accrual
-- ---------------------------------------------------------------------------
-- Settlement events (subgraph updates totalAmountReceived on these only):
--   SubscriptionApproved, SubscriptionRevoked, SubscriptionDistributionClaimed,
--   SubscriptionUnitsUpdated
-- Approval flag: Approve/Revoke only.
-- Subscribed flag: IndexSubscribed / IndexUnsubscribed (was ignored before).

CREATE OR REPLACE VIEW api_index_subscriptions AS
SELECT
    k.subscriber AS subscriber,
    k.publisher AS publisher,
    k.token AS token,
    k.index_id AS index_id,
    concat(k.subscriber, '-', k.publisher, '-', k.token, '-', k.index_id) AS subscription_id,
    ifNull(u.units, toInt256(0)) AS units,
    ifNull(a.approved, 0) AS approved,
    -- subscribed: prefer IndexSubscribed/Unsubscribed; fallback units>0
    multiIf(
        isNotNull(s.subscribed), s.subscribed,
        ifNull(u.units, toInt256(0)) > 0, 1,
        0
    ) AS subscribed,
    ifNull(r.total_amount_received_until_updated_at, toInt256(0))
        AS total_amount_received_until_updated_at,
    ifNull(r.index_value_until_updated_at, toInt256(0)) AS index_value_until_updated_at,
    -- Pending (unclaimed) as of latest index value — formula from subgraph schema
    ifNull(u.units, toInt256(0))
        * (
            ifNull(ix.index_value, toInt256(0))
            - ifNull(r.index_value_until_updated_at, toInt256(0))
        ) AS amount_pending,
    ifNull(u.units, toInt256(0))
        * (
            ifNull(ix.index_value, toInt256(0))
            - ifNull(r.index_value_until_updated_at, toInt256(0))
        )
        + ifNull(r.total_amount_received_until_updated_at, toInt256(0))
        AS total_amount_received_as_of_index,
    greatest(
        ifNull(u.units_updated_at, toUInt64(0)),
        ifNull(a.approval_updated_at, toUInt64(0)),
        ifNull(r.last_settled_at, toUInt64(0)),
        ifNull(s.subscribed_updated_at, toUInt64(0))
    ) AS updated_at,
    greatest(
        ifNull(u.units_updated_at_block, toUInt64(0)),
        ifNull(a.approval_updated_at_block, toUInt64(0)),
        ifNull(r.last_settled_at_block, toUInt64(0))
    ) AS updated_at_block
FROM v_ida_subscription_keys AS k
LEFT JOIN v_ida_subscription_units AS u
    ON k.subscriber = u.subscriber AND k.publisher = u.publisher
   AND k.token = u.token AND k.index_id = u.index_id
LEFT JOIN v_ida_subscription_approvals AS a
    ON k.subscriber = a.subscriber AND k.publisher = a.publisher
   AND k.token = a.token AND k.index_id = a.index_id
LEFT JOIN v_ida_subscription_subscribed AS s
    ON k.subscriber = s.subscriber AND k.publisher = s.publisher
   AND k.token = s.token AND k.index_id = s.index_id
LEFT JOIN v_ida_subscription_amount_received AS r
    ON k.subscriber = r.subscriber AND k.publisher = r.publisher
   AND k.token = r.token AND k.index_id = r.index_id
LEFT JOIN v_ida_index_value_latest AS ix
    ON k.publisher = ix.publisher AND k.token = ix.token AND k.index_id = ix.index_id;

-- Back-compat alias for prior approval-only view name
CREATE OR REPLACE VIEW api_index_subscription_approvals AS
SELECT
    subscriber,
    publisher,
    token,
    index_id,
    approved,
    updated_at AS approval_updated_at
FROM api_index_subscriptions;

-- ---------------------------------------------------------------------------
-- GDA: Pool / Member / Distributor HOL (P2 economics)
-- ---------------------------------------------------------------------------

-- Latest flow distribution state per pool (total pool flow rate)
CREATE OR REPLACE VIEW api_pools AS
SELECT
    p.pool AS pool,
    p.token AS token,
    p.admin AS admin,
    p.created_at AS created_at,
    p.created_at_block AS created_at_block,
    ifNull(u.total_units, toInt256(0)) AS total_units,
    ifNull(u.total_connected_units, toInt256(0)) AS total_connected_units,
    ifNull(u.total_disconnected_units, toInt256(0)) AS total_disconnected_units,
    ifNull(u.total_members, 0) AS total_members,
    ifNull(u.total_connected_members, 0) AS total_connected_members,
    ifNull(u.total_disconnected_members, 0) AS total_disconnected_members,
    ifNull(f.flow_rate, toInt256(0)) AS flow_rate,
    ifNull(f.adjustment_flow_rate, toInt256(0)) AS adjustment_flow_rate,
    ifNull(b.total_buffer, toInt256(0)) AS total_buffer,
    ifNull(i.total_amount_instantly_distributed_until_updated_at, toInt256(0))
        AS total_amount_instantly_distributed_until_updated_at,
    -- Closed flow periods only (between FlowDistributionUpdated events)
    ifNull(fd.total_amount_flowed_distributed_until_updated_at, toInt256(0))
        AS total_amount_flowed_distributed_until_updated_at,
    ifNull(i.total_amount_instantly_distributed_until_updated_at, toInt256(0))
        + ifNull(fd.total_amount_flowed_distributed_until_updated_at, toInt256(0))
        AS total_amount_distributed_until_updated_at,
    -- per-unit rate (integer division like subgraph divideOrZero)
    if(
        ifNull(u.total_units, toInt256(0)) = 0,
        toInt256(0),
        intDiv(ifNull(f.flow_rate, toInt256(0)), u.total_units)
    ) AS per_unit_flow_rate,
    ifNull(f.updated_at, toUInt64(0)) AS last_flow_updated_at,
    greatest(
        p.created_at,
        ifNull(f.updated_at, toUInt64(0)),
        ifNull(b.updated_at, toUInt64(0)),
        ifNull(i.updated_at, toUInt64(0))
    ) AS updated_at,
    greatest(
        p.created_at_block,
        ifNull(f.updated_at_block, toUInt64(0)),
        ifNull(b.updated_at_block, toUInt64(0)),
        ifNull(i.updated_at_block, toUInt64(0))
    ) AS updated_at_block
FROM
(
    SELECT
        lower(pool) AS pool,
        lower(token) AS token,
        lower(admin) AS admin,
        min(block_timestamp) AS created_at,
        min(block_number) AS created_at_block
    FROM pool_created_events
    WHERE _deleted_ = false
    GROUP BY pool, token, admin
) AS p
LEFT JOIN v_gda_pool_unit_stats AS u ON p.pool = u.pool AND p.token = u.token
LEFT JOIN v_gda_pool_flow_latest AS f ON p.pool = f.pool AND p.token = f.token
LEFT JOIN v_gda_pool_buffer AS b ON p.pool = b.pool AND p.token = b.token
LEFT JOIN v_gda_pool_instant AS i ON p.pool = i.pool AND p.token = i.token
LEFT JOIN v_gda_pool_flowed AS fd ON p.pool = fd.pool AND p.token = fd.token;

-- Open-period flow accrual helper (wall clock). For fixed H:
--   flowed_until + flow_rate * (H_ts - last_flow_updated_at)
-- Note: alias must not be `pool` — conflicts with column name `pool`.
CREATE OR REPLACE VIEW api_pools_as_of_now AS
SELECT
    p.pool AS pool,
    p.token AS token,
    p.admin AS admin,
    p.created_at AS created_at,
    p.created_at_block AS created_at_block,
    p.total_units AS total_units,
    p.total_connected_units AS total_connected_units,
    p.total_disconnected_units AS total_disconnected_units,
    p.total_members AS total_members,
    p.total_connected_members AS total_connected_members,
    p.total_disconnected_members AS total_disconnected_members,
    p.flow_rate AS flow_rate,
    p.adjustment_flow_rate AS adjustment_flow_rate,
    p.total_buffer AS total_buffer,
    p.total_amount_instantly_distributed_until_updated_at AS total_amount_instantly_distributed_until_updated_at,
    p.total_amount_flowed_distributed_until_updated_at AS total_amount_flowed_distributed_until_updated_at,
    p.total_amount_distributed_until_updated_at AS total_amount_distributed_until_updated_at,
    p.per_unit_flow_rate AS per_unit_flow_rate,
    p.last_flow_updated_at AS last_flow_updated_at,
    p.updated_at AS updated_at,
    p.updated_at_block AS updated_at_block,
    if(
        p.flow_rate = 0 OR p.last_flow_updated_at = 0,
        p.total_amount_flowed_distributed_until_updated_at,
        p.total_amount_flowed_distributed_until_updated_at
            + p.flow_rate * toInt256(
                toInt64(toUnixTimestamp(now())) - toInt64(p.last_flow_updated_at)
            )
    ) AS total_amount_flowed_distributed_as_of_now,
    p.total_amount_instantly_distributed_until_updated_at
        + if(
            p.flow_rate = 0 OR p.last_flow_updated_at = 0,
            p.total_amount_flowed_distributed_until_updated_at,
            p.total_amount_flowed_distributed_until_updated_at
                + p.flow_rate * toInt256(
                    toInt64(toUnixTimestamp(now())) - toInt64(p.last_flow_updated_at)
                )
        ) AS total_amount_distributed_as_of_now
FROM api_pools AS p;

-- PoolDistributor HOL
CREATE OR REPLACE VIEW api_pool_distributors AS
SELECT
    k.pool AS pool,
    k.token AS token,
    k.distributor AS distributor,
    concat('poolDistributor-', k.pool, '-', k.distributor) AS pool_distributor_id,
    ifNull(f.flow_rate, toInt256(0)) AS flow_rate,
    ifNull(b.total_buffer, toInt256(0)) AS total_buffer,
    ifNull(i.total_amount_instantly_distributed_until_updated_at, toInt256(0))
        AS total_amount_instantly_distributed_until_updated_at,
    ifNull(fd.total_amount_flowed_distributed_until_updated_at, toInt256(0))
        AS total_amount_flowed_distributed_until_updated_at,
    ifNull(i.total_amount_instantly_distributed_until_updated_at, toInt256(0))
        + ifNull(fd.total_amount_flowed_distributed_until_updated_at, toInt256(0))
        AS total_amount_distributed_until_updated_at,
    greatest(
        ifNull(f.updated_at, toUInt64(0)),
        ifNull(b.updated_at, toUInt64(0)),
        ifNull(i.updated_at, toUInt64(0))
    ) AS updated_at,
    greatest(
        ifNull(f.updated_at_block, toUInt64(0)),
        ifNull(b.updated_at_block, toUInt64(0)),
        ifNull(i.updated_at_block, toUInt64(0))
    ) AS updated_at_block
FROM v_gda_distributor_keys AS k
LEFT JOIN v_gda_distributor_flow_latest AS f
    ON k.pool = f.pool AND k.token = f.token AND k.distributor = f.distributor
LEFT JOIN v_gda_distributor_buffer AS b
    ON k.pool = b.pool AND k.token = b.token AND k.distributor = b.distributor
LEFT JOIN v_gda_distributor_instant AS i
    ON k.pool = i.pool AND k.token = i.token AND k.distributor = i.distributor
LEFT JOIN v_gda_distributor_flowed AS fd
    ON k.pool = fd.pool AND k.token = fd.token AND k.distributor = fd.distributor;

-- Pool members (units + connection + claims)
CREATE OR REPLACE VIEW api_pool_members AS
SELECT
    m.pool AS pool,
    m.member AS member,
    m.token AS token,
    concat('poolMember-', m.pool, '-', m.member) AS pool_member_id,
    m.units AS units,
    ifNull(c.connected, 0) AS is_connected,
    ifNull(cl.total_amount_claimed, toInt256(0)) AS total_amount_claimed,
    m.updated_at AS updated_at,
    m.updated_at_block AS updated_at_block
FROM v_gda_member_units_latest AS m
LEFT JOIN v_gda_member_connected_latest AS c
    ON m.pool = c.pool AND m.member = c.member AND m.token = c.token
LEFT JOIN
(
    SELECT
        lower(contract) AS pool,
        lower(member) AS member,
        lower(token) AS token,
        argMax(toInt256OrZero(total_claimed), (block_number, log_index)) AS total_amount_claimed
    FROM distribution_claimed_events
    WHERE _deleted_ = false
    GROUP BY pool, member, token
) AS cl
    ON m.pool = cl.pool AND m.member = cl.member AND m.token = cl.token;

-- Members with connection (back-compat name used by README / examples)
CREATE OR REPLACE VIEW api_pool_members_enriched AS
SELECT
    pool,
    member,
    token,
    pool_member_id,
    units,
    is_connected AS connected,
    total_amount_claimed,
    updated_at,
    updated_at_block
FROM api_pool_members;

CREATE OR REPLACE VIEW api_pool_connections AS
SELECT
    pool,
    member,
    token,
    connected,
    updated_at
FROM v_gda_member_connected_latest;

-- ---------------------------------------------------------------------------
-- Accounts seen across protocol surfaces (events only)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW api_accounts AS
SELECT
    account,
    min(first_seen) AS first_seen,
    max(last_seen) AS last_seen
FROM
(
    SELECT lower(sender) AS account, min(block_timestamp) AS first_seen, max(block_timestamp) AS last_seen
    FROM flow_updated_events WHERE _deleted_ = false
    GROUP BY sender
    UNION ALL
    SELECT lower(receiver), min(block_timestamp), max(block_timestamp)
    FROM flow_updated_events WHERE _deleted_ = false
    GROUP BY receiver
    UNION ALL
    SELECT lower(admin), min(block_timestamp), max(block_timestamp)
    FROM pool_created_events WHERE _deleted_ = false
    GROUP BY admin
    UNION ALL
    SELECT lower(member), min(block_timestamp), max(block_timestamp)
    FROM member_units_updated_events WHERE _deleted_ = false
    GROUP BY member
    UNION ALL
    SELECT lower(publisher), min(block_timestamp), max(block_timestamp)
    FROM index_created_events WHERE _deleted_ = false
    GROUP BY publisher
    UNION ALL
    SELECT lower(subscriber), min(block_timestamp), max(block_timestamp)
    FROM subscription_units_updated_events WHERE _deleted_ = false
    GROUP BY subscriber
    UNION ALL
    SELECT lower(distributor), min(block_timestamp), max(block_timestamp)
    FROM flow_distribution_updated_events WHERE _deleted_ = false
    GROUP BY distributor
)
GROUP BY account;
