-- Example outcome queries (data from Substreams only). Addresses: lowercase hex, no 0x.
-- Public api_* views are defined under sql/hasura/ and applied after these internal views.

-- 1) All streams for an account
-- SELECT * FROM api_streams
-- WHERE sender = '…' OR receiver = '…' ORDER BY updated_at DESC;

-- 2) Open streams + live streamed estimate (wall clock — not for parity)
-- SELECT stream_id, token, sender, receiver, current_flow_rate, deposit, streamed_as_of_now
-- FROM api_streams WHERE is_open = 1;

-- 2b) Streamed amount at fixed unix timestamp H (parity / as-of-block)
-- SELECT stream_id, current_flow_rate, deposit,
--   streamed_amount_as_of(streamed_until_updated_at, current_flow_rate, updated_at, toUInt64(1700000000), is_open)
--     AS streamed_as_of_H
-- FROM v_streams_current WHERE is_open = 1;

-- 3) Account × token CFA stats
-- SELECT * FROM api_account_token_cfa_stats WHERE account = '…';

-- 4) Stream event history
-- SELECT block_number, event_type, flow_rate, revision, stream_id, deposit
-- FROM v_flow_updates_enriched
-- WHERE stream_pair_key = 'sender-receiver-token'
-- ORDER BY block_number, log_index;

-- 5) SuperTokens (from factory events in the backfill window)
-- SELECT * FROM api_tokens ORDER BY created_at_block;

-- 6) IDA indexes / subscriptions (P2: amount received + approval + subscribed)
-- SELECT index_entity_id, index_value, total_units, total_amount_distributed_until_updated_at
-- FROM api_indexes LIMIT 100;
-- SELECT subscription_id, units, approved, subscribed,
--        total_amount_received_until_updated_at, amount_pending, total_amount_received_as_of_index
-- FROM api_index_subscriptions WHERE units > 0 LIMIT 100;

-- 7) GDA pools / members / distributors (P2 economics)
-- SELECT pool, flow_rate, total_buffer, total_units,
--        total_amount_instantly_distributed_until_updated_at,
--        total_amount_flowed_distributed_until_updated_at,
--        total_amount_distributed_until_updated_at
-- FROM api_pools;
-- SELECT * FROM api_pool_members WHERE units > 0;
-- SELECT * FROM api_pool_distributors WHERE flow_rate > 0 OR total_buffer > 0;
