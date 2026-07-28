# Hasura over ClickHouse (Uniswap v4)

Public GraphQL surface for the Uniswap v4 package.

## Prereqs

1. `docker compose up -d` (ClickHouse + Hasura + connector)
2. Sink running until event tables exist
3. `./scripts/apply_views.sh` (creates `api_*` views)
4. `./scripts/hasura_connect_clickhouse.sh` (registers source + tracks views)

## Endpoints

| Service | URL |
|---------|-----|
| GraphQL | http://localhost:8080/v1/graphql |
| Console | http://localhost:8080/console |
| Admin secret | `SecureMe!` (override with `HASURA_GRAPHQL_ADMIN_SECRET`) |

Root fields: `{database}_{view}` → e.g. `uniswap_v4_api_pools`.

See [examples.graphql](./examples.graphql).
