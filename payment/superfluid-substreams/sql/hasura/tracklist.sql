-- Hasura track list — views defined in 01_api_cfa.sql / 02_api_ida_gda.sql
--
-- Layout:
--   sql/clickhouse/     → internal v_* HOL (not tracked by default)
--   sql/hasura/*.sql    → api_* CREATE VIEW (applied by apply_views.sh)
--   sql/hasura/tracklist.sql → this file (parsed by hasura_connect_clickhouse.sh)
--
-- GraphQL root fields: {database}_{view} e.g. superfluid_api_streams

track api_streams
track api_streams_for_account
track api_account_token_cfa_stats
track api_flow_operators
track api_tokens
track api_indexes
track api_index_subscriptions
track api_pools
track api_pools_as_of_now
track api_pool_members
track api_pool_distributors
track api_accounts
