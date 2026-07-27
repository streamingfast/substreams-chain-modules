#!/usr/bin/env bash
# Compare CFA open-stream rates: ClickHouse api_streams vs Superfluid subgraph.
# Requires: curl, bash. Optional: jq (for JSON parsing; required for full compare).
#
# Env:
#   SUPERFLUID_SUBGRAPH  default base-mainnet protocol-v1 endpoint
#   CLICKHOUSE_HTTP / CLICKHOUSE_USER / CLICKHOUSE_PASSWORD / CLICKHOUSE_DB
#   PARITY_LIMIT         default 50

set -euo pipefail

SG="${SUPERFLUID_SUBGRAPH:-https://subgraph-endpoints.superfluid.dev/base-mainnet/protocol-v1}"
CH_HTTP="${CLICKHOUSE_HTTP:-http://127.0.0.1:8123}"
CH_USER="${CLICKHOUSE_USER:-default}"
CH_PASS="${CLICKHOUSE_PASSWORD:-SecureMe!}"
DB="${CLICKHOUSE_DB:-superfluid}"
LIMIT="${PARITY_LIMIT:-50}"

if ! command -v jq >/dev/null 2>&1; then
  echo "parity_cfa.sh needs jq for JSON parsing. Install: brew install jq" >&2
  exit 1
fi

ch() {
  curl -sS -u "${CH_USER}:${CH_PASS}" --data-binary "$1" "${CH_HTTP}/?database=${DB}"
}

gql() {
  local q="$1"
  curl -sS -X POST "$SG" \
    -H 'content-type: application/json' \
    -H 'user-agent: superfluid-parity-cfa/0.3' \
    --data-binary "$(jq -n --arg q "$q" '{query:$q}')"
}

echo "CH: $CH_HTTP db=$DB"
echo "Subgraph: $SG"
echo "Limit: $LIMIT"

# Open streams sample (TSV: sender receiver token rate deposit)
ROWS_FILE="$(mktemp)"
ch "
SELECT
  sender,
  receiver,
  token,
  toString(current_flow_rate) AS rate,
  toString(deposit) AS deposit
FROM api_streams
WHERE is_open = 1
LIMIT ${LIMIT}
FORMAT TabSeparated
" >"$ROWS_FILE"

if [[ ! -s "$ROWS_FILE" ]]; then
  rm -f "$ROWS_FILE"
  echo "No open streams in api_streams — is the sink populated and views applied?"
  exit 1
fi

n_rows="$(wc -l <"$ROWS_FILE" | tr -d ' ')"
echo "Comparing ${n_rows} open streams from CH..."

ok=0
fail=0
dep_ok=0
dep_fail=0

while IFS=$'\t' read -r sender receiver token rate deposit; do
  q=$(cat <<EOF
{
  streams(
    where: {
      sender: "0x${sender}"
      receiver: "0x${receiver}"
      token: "0x${token}"
      currentFlowRate_gt: "0"
    }
    first: 5
  ) { id currentFlowRate deposit }
}
EOF
)
  resp="$(gql "$q")"
  if echo "$resp" | jq -e '.errors' >/dev/null 2>&1; then
    echo "GQL error for ${sender:0:8}…→${receiver:0:8}…"
    echo "$resp" | jq -c '.errors' 2>/dev/null || echo "$resp"
    fail=$((fail + 1))
    continue
  fi
  n="$(echo "$resp" | jq '.data.streams | length')"
  if [[ "$n" == "0" || "$n" == "null" ]]; then
    echo "FAIL missing on SG ${sender:0:8}…→${receiver:0:8}… rate=$rate"
    fail=$((fail + 1))
    continue
  fi
  if echo "$resp" | jq -e --arg r "$rate" '[.data.streams[].currentFlowRate] | index($r) != null' >/dev/null; then
    ok=$((ok + 1))
  else
    sg_rates="$(echo "$resp" | jq -c '[.data.streams[].currentFlowRate]')"
    echo "FAIL rate ch=$rate sg=$sg_rates ${sender:0:8}…→${receiver:0:8}…"
    fail=$((fail + 1))
  fi
  if [[ -n "$deposit" && "$deposit" != "\\N" && "$deposit" != "0" ]]; then
    if echo "$resp" | jq -e --arg d "$deposit" '[.data.streams[].deposit | tostring] | index($d) != null' >/dev/null; then
      dep_ok=$((dep_ok + 1))
    else
      sg_dep="$(echo "$resp" | jq -c '[.data.streams[].deposit]')"
      echo "FAIL deposit ch=$deposit sg=$sg_dep ${sender:0:8}…→${receiver:0:8}…"
      dep_fail=$((dep_fail + 1))
    fi
  fi
done <"$ROWS_FILE"
rm -f "$ROWS_FILE"

# Registry counts
echo
echo "CH registry counts:"
for label_sql in \
  "api_tokens:SELECT count() FROM api_tokens" \
  "api_indexes:SELECT count() FROM api_indexes" \
  "api_pools:SELECT count() FROM api_pools" \
  "api_index_subscriptions:SELECT count() FROM api_index_subscriptions" \
  "api_pool_members:SELECT count() FROM api_pool_members"
do
  label="${label_sql%%:*}"
  sql="${label_sql#*:}"
  echo "  $label: $(ch "$sql" | tr -d '\n')"
done

echo
echo "Stream rate parity: $ok ok, $fail fail (of $((ok + fail)) compared)"
echo "Stream deposit parity: $dep_ok ok, $dep_fail fail"
if [[ "$fail" -eq 0 && "$ok" -gt 0 && "$dep_fail" -eq 0 ]]; then
  exit 0
fi
exit 1
