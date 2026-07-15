#!/usr/bin/env bash
# Per-crate line-coverage gate for the cryptographic crates.
#
# Reads a cargo-llvm-cov JSON export (llvm-cov export format), aggregates
# line counts per workspace crate, and fails when any of the crates that
# hold cryptographic logic (domain, application, infrastructure) falls
# below the threshold. The bin crate hosts the command-line interface and
# the composition root, so it is reported but not gated.
#
# Usage:
#   cargo llvm-cov --workspace --json --summary-only \
#       --output-path coverage.json
#   scripts/coverage-gate.sh coverage.json
set -euo pipefail

THRESHOLD=90
GATED_CRATES=(domain application infrastructure)

if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "usage: $0 <cargo-llvm-cov-json-export>" >&2
    exit 2
fi

report="$1"

# One "<crate> <line count> <lines covered>" row per source file.
rows=$(python3 - "$report" <<'PY'
import json
import sys

with open(sys.argv[1]) as handle:
    export = json.load(handle)

for entry in export["data"][0]["files"]:
    filename = entry["filename"]
    marker = "/crates/"
    position = filename.find(marker)
    if position == -1:
        continue
    crate = filename[position + len(marker):].split("/", 1)[0]
    lines = entry["summary"]["lines"]
    print(crate, lines["count"], lines["covered"])
PY
)

failed=0
for crate in "${GATED_CRATES[@]}" bin; do
    total=0
    covered=0
    while read -r name count cov; do
        if [ "$name" = "$crate" ]; then
            total=$((total + count))
            covered=$((covered + cov))
        fi
    done <<<"$rows"

    if [ "$total" -eq 0 ]; then
        echo "error: no coverage data found for crate '$crate'" >&2
        failed=1
        continue
    fi

    percent=$((100 * covered / total))
    gated=no
    for gated_crate in "${GATED_CRATES[@]}"; do
        [ "$crate" = "$gated_crate" ] && gated=yes
    done

    printf '%-16s %5d/%5d lines  %3d%%  (gate: %s)\n' \
        "$crate" "$covered" "$total" "$percent" \
        "$([ "$gated" = yes ] && echo ">=${THRESHOLD}%" || echo none)"

    if [ "$gated" = yes ] && [ "$percent" -lt "$THRESHOLD" ]; then
        echo "error: crate '$crate' is below the ${THRESHOLD}% line-coverage gate" >&2
        failed=1
    fi
done

exit "$failed"
