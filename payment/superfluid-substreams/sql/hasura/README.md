# Hasura GraphQL surface

Public **`api_*`** views live here. Internal HOL (`v_*`) stays under `../clickhouse/`.

| Path | Role |
|------|------|
| **`01_api_cfa.sql`** | `api_streams`, flow operators, CFA stats |
| **`02_api_ida_gda.sql`** | tokens, IDA, GDA pools/members/distributors, accounts |
| **`tracklist.sql`** | Which views Hasura tracks (parsed by `scripts/hasura_connect_clickhouse.sh`) |
| **`examples.graphql`** | Sample GraphQL queries |

## Apply

```bash
# Internal + api_* (order is automatic)
./scripts/apply_views.sh

# Register / track in Hasura
./scripts/hasura_connect_clickhouse.sh
```

Edit **view SQL** → change GraphQL fields after re-apply + re-track.  
Edit **tracklist.sql** → change which views appear in GraphQL.

Console: http://localhost:8080/console  
GraphQL: http://localhost:8080/v1/graphql  

## Console error: `'' is not a valid JavaScript MIME type`

The Console SPA **dynamic-imports** `assetLoader.js.gz`. Safari/WebKit (and some blocked-CDN cases) fail with:

```text
TypeError: '' is not a valid JavaScript MIME type
```

**Fix (already in `docker-compose.yml`):** serve assets from the image, not the Hasura CDN:

```yaml
HASURA_GRAPHQL_CONSOLE_ASSETS_DIR: /srv/console-assets
```

```bash
docker compose up -d hasura
# hard-refresh browser or use a private window
```

If it still fails in Safari, use Chrome/Firefox, or query GraphQL without the Console:

```bash
curl -s http://localhost:8080/v1/graphql \
  -H 'content-type: application/json' \
  -H 'x-hasura-admin-secret: SecureMe!' \
  -d '{"query":"{ superfluid_api_streams(limit:2){ stream_id current_flow_rate } }"}'
```
