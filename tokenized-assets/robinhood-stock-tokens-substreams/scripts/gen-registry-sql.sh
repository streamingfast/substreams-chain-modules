#!/usr/bin/env bash
# Regenerates the stock_registry INSERT block in schema.clickhouse.sql from
# data/registry-4663.tsv (ticker, token, multiplier, decimals, status, name)
# joined with data/feed-token-map.tsv (ticker, token, proxy, aggregator) on
# token. Replaces the block between the BEGIN/END GENERATED markers in place.
set -euo pipefail
cd "$(dirname "$0")/.."

SNAPSHOT_DATE="2026-09-05"
SCHEMA="schema.clickhouse.sql"
REGISTRY="data/registry-4663.tsv"
FEEDS="data/feed-token-map.tsv"
BEGIN_MARK="-- BEGIN GENERATED STOCK_REGISTRY ROWS"
END_MARK="-- END GENERATED STOCK_REGISTRY ROWS"

join_awk=$(mktemp)
trap 'rm -f "$join_awk" "$rows_file"' EXIT

cat > "$join_awk" <<'AWK_EOF'
function q(s) {
    gsub(/'/, "''", s)
    return "'" s "'"
}
FNR == NR {
    proxy[$2] = $3
    agg[$2] = $4
    next
}
{
    n++
    ticker = $1; token = $2; mult = $3; decimals = $4; status = $5; name = $6
    has_feed = (token in proxy) ? "true" : "false"
    p = (token in proxy) ? proxy[token] : ""
    a = (token in agg) ? agg[token] : ""
    rows[n] = sprintf("    (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s)", q(ticker), q(token), q(p), q(a), q(mult), decimals, q(status), q(name), has_feed, q(snap))
}
END {
    print "INSERT INTO stock_registry (ticker, token, feed, aggregator, multiplier_str, decimals, status, name, has_feed, snapshot_date) VALUES"
    for (i = 1; i <= n; i++) {
        sep = (i == n) ? ";" : ","
        print rows[i] sep
    }
}
AWK_EOF

rows_file=$(mktemp)
awk -F'\t' -v snap="$SNAPSHOT_DATE" -f "$join_awk" "$FEEDS" "$REGISTRY" > "$rows_file"

replace_awk=$(mktemp)
trap 'rm -f "$join_awk" "$rows_file" "$replace_awk"' EXIT
cat > "$replace_awk" <<'AWK_EOF'
BEGIN {
    while ((getline line < rows_file) > 0) {
        rows = rows line "\n"
    }
}
$0 == begin {
    print
    printf "%s", rows
    skip = 1
    next
}
$0 == end {
    skip = 0
    print
    next
}
skip { next }
{ print }
AWK_EOF

tmp=$(mktemp)
trap 'rm -f "$join_awk" "$rows_file" "$replace_awk" "$tmp"' EXIT
awk -v begin="$BEGIN_MARK" -v end="$END_MARK" -v rows_file="$rows_file" -f "$replace_awk" "$SCHEMA" > "$tmp"
mv "$tmp" "$SCHEMA"
