#!/usr/bin/env bash
# check-logging.sh — the M-log gate: svara must build warning-free and pass its
# suite BOTH with and without `-D LOGGING`.
#
# This exists because `cyrius test` does not forward `-D`, so the ordinary
# runner (`cyrius tests tests`) only ever exercises the logging-off half of
# tests/logging.tcyr. Without this script the entire `#ifdef LOGGING` half of
# the codebase — the module, and the guarded blocks threaded through four
# entry points — would be compiled by nobody.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

fail=0

# A build is only clean if it emits no `warning:` at all. The `note:` about
# unreachable fns is DCE information, not a warning, and is expected.
build() {
    local label="$1"; shift
    local log="$OUT/${label//\//-}.log"
    if ! cyrius build "$@" > "$log" 2>&1; then
        echo "FAIL  $label: build failed"; sed 's/^/        /' "$log"; return 1
    fi
    if grep -q "^warning:" "$log"; then
        echo "FAIL  $label: build emitted warnings"
        grep "^warning:" "$log" | sed 's/^/        /'
        return 1
    fi
    echo "ok    $label: builds clean"
}

run() {
    local label="$1"; local bin="$2"
    local log="$OUT/${label//\//-}.run"
    if ! "$bin" > "$log" 2>&1; then
        echo "FAIL  $label: suite failed"; grep -vE "^\[[0-9]+\]" "$log" | tail -20 | sed 's/^/        /'
        return 1
    fi
    echo "ok    $label: $(grep -oE '[0-9]+ passed, [0-9]+ failed' "$log" | tail -1)"
}

echo "== logging OFF =="
build "off/lib"   src/main.cyr        "$OUT/svara-off"      || fail=1
build "off/suite" tests/logging.tcyr  "$OUT/logging-off"    || fail=1
run   "off/suite" "$OUT/logging-off"                         || fail=1

echo "== logging ON (-D LOGGING) =="
build "on/lib"    -D LOGGING src/main.cyr       "$OUT/svara-on"    || fail=1
build "on/suite"  -D LOGGING tests/logging.tcyr "$OUT/logging-on"  || fail=1
run   "on/suite"  "$OUT/logging-on"                                 || fail=1

# The hot path must stay clean: no log call may appear on the per-sample
# synthesis path. Assert it structurally rather than by inspection — the
# per-sample functions must contain no svara_log_/svara_span_ call.
echo "== hot path =="
hot='svara_glottal_next_sample|svara_tract_process_sample|svara_formant_bank_process|svara_formant_filter_process_sample|svara_dc_blocker_process|svara_rng_next_u32'
if awk -v pat="$hot" '
    $0 ~ "^fn (" pat ")\\(" { inside=1 }
    inside && /^}/           { inside=0 }
    inside && /svara_(log|span)_/ { print FILENAME ":" FNR ": " $0; found=1 }
    END { exit(found ? 1 : 0) }
' src/*.cyr; then
    echo "ok    no log or span call on the per-sample path"
else
    echo "FAIL  a log/span call appears on the per-sample path"; fail=1
fi

[ "$fail" -eq 0 ] && echo "== M-log gate: PASS ==" || { echo "== M-log gate: FAIL =="; exit 1; }
