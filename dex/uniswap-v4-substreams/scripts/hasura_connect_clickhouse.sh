#!/usr/bin/env bash
# Register ClickHouse as a Hasura source and track views from sql/hasura/tracklist.sql.
# Requires: curl, bash. Optional: jq (prettier errors). No Python.
#
# Prereqs: docker compose up -d && ./scripts/apply_views.sh
#
# Env: HASURA_ENDPOINT, HASURA_ADMIN_SECRET, CLICKHOUSE_URL, CLICKHOUSE_USER,
#      CLICKHOUSE_PASSWORD, CLICKHOUSE_DATABASE, SOURCE_NAME, HASURA_TRACKLIST

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HASURA="${HASURA_ENDPOINT:-http://127.0.0.1:8080}"
HASURA="${HASURA%/}"
SECRET="${HASURA_ADMIN_SECRET:-SecureMe!}"
SOURCE="${SOURCE_NAME:-uniswap_v4_ch}"
CH_URL="${CLICKHOUSE_URL:-http://clickhouse:8123}"
CH_URL="${CH_URL%/}"
CH_USER="${CLICKHOUSE_USER:-default}"
CH_PASS="${CLICKHOUSE_PASSWORD:-SecureMe!}"
CH_DB="${CLICKHOUSE_DATABASE:-uniswap_v4}"
TRACKLIST="${HASURA_TRACKLIST:-$ROOT/sql/hasura/tracklist.sql}"

metadata() {
  local type="$1"
  local args_json="$2"
  local body http_code tmp
  body="$(printf '{"type":%s,"args":%s}' "$(json_str "$type")" "$args_json")"
  tmp="$(mktemp)"
  http_code="$(curl -sS -o "$tmp" -w '%{http_code}' \
    -X POST "${HASURA}/v1/metadata" \
    -H "Content-Type: application/json" \
    -H "x-hasura-admin-secret: ${SECRET}" \
    --data-binary "$body")"
  if [[ "$http_code" != "200" ]]; then
    echo "metadata $type failed HTTP $http_code:" >&2
    cat "$tmp" >&2
    echo >&2
    rm -f "$tmp"
    return 1
  fi
  cat "$tmp"
  rm -f "$tmp"
}

# Minimal JSON string escape
json_str() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  printf '"%s"' "$s"
}

load_tracklist() {
  local path="$1" line name low
  if [[ ! -f "$path" ]]; then
    echo "tracklist not found: $path" >&2
    return 1
  fi
  while IFS= read -r line || [[ -n "$line" ]]; do
    # trim
    line="$(printf '%s' "$line" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
    [[ -z "$line" || "$line" == --* ]] && continue
    low="$(printf '%s' "$line" | tr '[:upper:]' '[:lower:]')"
    case "$low" in
      track\ *)
        name="${line#* }"
        name="$(printf '%s' "$name" | awk '{print $1}')"
        [[ -n "$name" ]] && echo "$name"
        ;;
    esac
  done <"$path"
}

echo "Hasura: $HASURA"
echo "ClickHouse URL (from Hasura): $CH_URL  schema/db=$CH_DB"
TABLES=()
while IFS= read -r _t; do
  [[ -n "$_t" ]] && TABLES+=("$_t")
done < <(load_tracklist "$TRACKLIST")
if [[ ${#TABLES[@]} -eq 0 ]]; then
  echo "no track lines in $TRACKLIST" >&2
  exit 1
fi
echo "Tracklist: $TRACKLIST (${#TABLES[@]} views)"

CONFIG_JSON="$(printf '{"url":%s,"username":%s,"password":%s}' \
  "$(json_str "$CH_URL")" "$(json_str "$CH_USER")" "$(json_str "$CH_PASS")")"

ADD_ARGS="$(printf '{"name":%s,"configuration":%s,"replace_configuration":true}' \
  "$(json_str "$SOURCE")" "$CONFIG_JSON")"

if ! out="$(metadata clickhouse_add_source "$ADD_ARGS" 2>/tmp/hasura_err.txt)"; then
  err="$(cat /tmp/hasura_err.txt 2>/dev/null || true)"
  if echo "$err" | grep -qi 'already exists'; then
    UPD_ARGS="$(printf '{"name":%s,"configuration":%s,"replace_configuration":true}' \
      "$(json_str "$SOURCE")" "$CONFIG_JSON")"
    metadata clickhouse_update_source "$UPD_ARGS" >/dev/null
    echo "OK updated source $SOURCE"
  else
    echo "HINT: Console → Data → Connect Database → ClickHouse" >&2
    echo "  url: $CH_URL  username: $CH_USER" >&2
    cat /tmp/hasura_err.txt >&2 || true
    exit 1
  fi
else
  echo "OK source $SOURCE"
fi

tracked=0
for table in "${TABLES[@]}"; do
  # TableName is JSON array [schema, name]
  TRACK_ARGS="$(printf '{"source":%s,"table":[%s,%s]}' \
    "$(json_str "$SOURCE")" "$(json_str "$CH_DB")" "$(json_str "$table")")"
  if metadata clickhouse_track_table "$TRACK_ARGS" >/dev/null 2>/tmp/hasura_track_err.txt; then
    echo "  track ${CH_DB}.${table}"
    tracked=$((tracked + 1))
  else
    err="$(cat /tmp/hasura_track_err.txt 2>/dev/null || true)"
    if echo "$err" | grep -qiE 'already tracked|already-tracked'; then
      echo "  already tracked $table"
      tracked=$((tracked + 1))
    else
      echo "  SKIP $table: $(echo "$err" | head -c 220)"
    fi
  fi
done

echo
echo "Tracked ${tracked}/${#TABLES[@]}."
echo "Console:  ${HASURA}/console"
echo "GraphQL:  ${HASURA}/v1/graphql"
echo "Admin secret: ${SECRET}"
echo "Examples: $ROOT/sql/hasura/examples.graphql"
