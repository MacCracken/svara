#!/usr/bin/env bash
# bench-history.sh — Run the Cyrius hot-path benchmarks and append results to
# a history log so per-op timings can be tracked for regressions across commits.
#
# Parses the `cyrius bench` report format (from lib/bench.cyr's bench_report):
#   "  <name>: <avg><unit> avg (min=<> max=<>) [<n> iters]"
# where <unit> is one of ns / us / ms / s (ASCII, no space before the unit).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

BENCH_FILE="${1:-benches/hotpath.bcyr}"
HISTORY_FILE="benches/history.csv"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_REV=$(git rev-parse --short HEAD 2>/dev/null || echo "uncommitted")

mkdir -p "$(dirname "$HISTORY_FILE")"
if [ ! -f "$HISTORY_FILE" ]; then
    echo "timestamp,git_rev,benchmark,avg_us" > "$HISTORY_FILE"
fi

# Normalize an avg value + unit to microseconds (printed with 3 decimals).
to_us() {
    awk -v v="$1" -v u="$2" 'BEGIN {
        if (u == "ns") v = v / 1000;
        else if (u == "ms") v = v * 1000;
        else if (u == "s")  v = v * 1000000;
        printf "%.3f", v;
    }'
}

recorded=0
while IFS= read -r line; do
    # "  <name>: <avg><unit> avg (...)" — name has no colon; unit is ASCII.
    if [[ "$line" =~ ^[[:space:]]+(.+):\ ([0-9.]+)(ns|us|ms|s)\ avg ]]; then
        name="${BASH_REMATCH[1]}"
        us=$(to_us "${BASH_REMATCH[2]}" "${BASH_REMATCH[3]}")
        # Bench names carry no commas (guarded by the definitions); safe unquoted.
        echo "${TIMESTAMP},${GIT_REV},${name},${us}" >> "$HISTORY_FILE"
        recorded=$((recorded + 1))
    fi
done < <(cyrius bench "$BENCH_FILE" 2>&1)

if [ "$recorded" -eq 0 ]; then
    echo "bench-history: no benchmark lines parsed from '$BENCH_FILE'" >&2
    exit 1
fi

echo "Recorded $recorded benchmark(s) to $HISTORY_FILE"
column -t -s, "$HISTORY_FILE"
