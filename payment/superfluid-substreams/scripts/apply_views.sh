#!/usr/bin/env bash
# Apply Superfluid ClickHouse views then Hasura api_* surface views.
# Requires: curl, bash. No Python.
#
# Order:
#   1) sql/clickhouse/0*.sql  — internal L0–L3 (v_*)
#   2) sql/hasura/0*.sql      — public api_*
#
# Env:
#   CLICKHOUSE_HTTP   default http://127.0.0.1:8123
#   CLICKHOUSE_USER   default default
#   CLICKHOUSE_PASSWORD default SecureMe!
#   CLICKHOUSE_DB     default superfluid

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CH_SQL="$ROOT/sql/clickhouse"
HASURA_SQL="$ROOT/sql/hasura"
CH_HTTP="${CLICKHOUSE_HTTP:-http://127.0.0.1:8123}"
CH_USER="${CLICKHOUSE_USER:-default}"
CH_PASS="${CLICKHOUSE_PASSWORD:-SecureMe!}"
DB="${CLICKHOUSE_DB:-superfluid}"

exec_sql() {
  local sql="$1"
  local out err
  out="$(curl -sS -u "${CH_USER}:${CH_PASS}" \
    --data-binary "$sql" \
    "${CH_HTTP}/?database=${DB}" 2>&1)" || {
    echo "ERROR executing SQL:" >&2
    echo "$sql" | head -c 400 >&2
    echo >&2
    echo "$out" >&2
    return 1
  }
  # ClickHouse returns error body with HTTP 200 sometimes — detect Code:
  if [[ "$out" == Code:* ]] || [[ "$out" == *"DB::Exception"* ]]; then
    echo "ERROR from ClickHouse:" >&2
    echo "$out" >&2
    echo "--- SQL (prefix) ---" >&2
    echo "$sql" | head -c 400 >&2
    echo >&2
    return 1
  fi
  printf '%s' "$out"
}

# Split file into statements ending with ';' (skip full-line -- comments).
apply_file() {
  local path="$1"
  local stmt="" line name
  while IFS= read -r line || [[ -n "$line" ]]; do
    # skip pure comment lines
    if [[ "$line" =~ ^[[:space:]]*-- ]]; then
      continue
    fi
    stmt+="$line"$'\n'
    if [[ "$line" =~ \;[[:space:]]*$ ]]; then
      # trim
      local s
      s="$(printf '%s' "$stmt" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
      stmt=""
      [[ -z "$s" ]] && continue
      exec_sql "$s" >/dev/null
      name="$(printf '%s' "$s" | awk '{
        for (i=1;i<=NF;i++) {
          if ($i!="CREATE" && $i!="OR" && $i!="REPLACE" && $i!="VIEW" && $i!="TABLE" && $i!="FUNCTION") {
            print $i; exit
          }
        }
      }')"
      echo "  OK ${name:-?}"
    fi
  done <"$path"
}

apply_dir() {
  local dir="$1"
  local label="$2"
  local f base
  shopt -s nullglob
  local files=("$dir"/0*.sql)
  shopt -u nullglob
  if ((${#files[@]} == 0)); then
    echo "(no 0*.sql in $dir)"
    return 0
  fi
  # sort for stable order
  IFS=$'\n' files=($(printf '%s\n' "${files[@]}" | sort))
  echo "== ${label} (${dir#"$ROOT"/}) =="
  for f in "${files[@]}"; do
    base="$(basename "$f")"
    case "$base" in
      *example*|*notes*|*tracklist*) continue ;;
    esac
    echo "apply $base"
    apply_file "$f"
  done
}

echo "database: $DB"
apply_dir "$CH_SQL" "ClickHouse internal views"
apply_dir "$HASURA_SQL" "Hasura api_* surface"
n="$(exec_sql "SELECT count() FROM system.tables WHERE database='${DB}' AND engine LIKE '%View%'")"
echo "views present: $n"
