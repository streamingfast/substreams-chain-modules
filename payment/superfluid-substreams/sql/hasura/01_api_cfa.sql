-- Hasura GraphQL API surface — CFA (api_*)
-- Depends on sql/clickhouse/01_views_cfa.sql (v_streams_current, streamed_amount_as_of, …)
-- Applied by scripts/apply_views.sh after clickhouse views.

CREATE OR REPLACE VIEW api_streams AS
SELECT
    stream_id,
    stream_pair_key,
    token,
    sender,
    receiver,
    current_flow_rate,
    deposit,
    user_data,
    opened_at,
    opened_at_block,
    updated_at,
    updated_at_block,
    closed_at,
    is_open,
    streamed_until_updated_at,
    -- Live wall-clock estimate only (non-deterministic). Prefer
    -- streamed_amount_as_of(..., <block_H_ts>, is_open) for parity at fixed H.
    streamed_amount_as_of(
        streamed_until_updated_at,
        current_flow_rate,
        updated_at,
        toUInt64(toUnixTimestamp(now())),
        is_open
    ) AS streamed_as_of_now
FROM v_streams_current;

-- All streams for an account (in + out). Filter: WHERE account = lower('...')
CREATE OR REPLACE VIEW api_streams_for_account AS
SELECT
    s.*,
    if(s.sender = s.receiver, 'self', 'participant') AS role_hint
FROM api_streams AS s;

CREATE OR REPLACE VIEW api_account_token_cfa_stats AS
SELECT
    account,
    token,
    countIf(is_open = 1 AND sender = account) AS active_out_streams,
    countIf(is_open = 1 AND receiver = account) AS active_in_streams,
    sumIf(current_flow_rate, is_open = 1 AND sender = account) AS total_out_rate,
    sumIf(current_flow_rate, is_open = 1 AND receiver = account) AS total_in_rate,
    sumIf(current_flow_rate, is_open = 1 AND receiver = account)
        - sumIf(current_flow_rate, is_open = 1 AND sender = account) AS net_rate
FROM
(
    SELECT
        sender AS account,
        token,
        sender,
        receiver,
        current_flow_rate,
        is_open
    FROM api_streams
    UNION ALL
    SELECT
        receiver AS account,
        token,
        sender,
        receiver,
        current_flow_rate,
        is_open
    FROM api_streams
    WHERE sender != receiver
)
GROUP BY account, token;

-- Flow operators: latest permissions per (operator, sender, token)
CREATE OR REPLACE VIEW api_flow_operators AS
SELECT
    lower(flow_operator) AS flow_operator,
    lower(sender) AS sender,
    lower(token) AS token,
    argMax(permissions, (block_number, log_index)) AS permissions,
    argMax(flow_rate_allowance, (block_number, log_index)) AS flow_rate_allowance,
    max(block_number) AS updated_at_block,
    max(block_timestamp) AS updated_at
FROM flow_operator_updated_events
WHERE _deleted_ = false
GROUP BY flow_operator, sender, token;
