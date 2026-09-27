#!/usr/bin/env bash
# Persistent per-runner caches; see docs/adr/0052-owned-ci-runners.md.
set -euo pipefail
: "${GITHUB_ENV:?GitHub Actions environment file required}"
: "${GITHUB_WORKSPACE:?GitHub Actions workspace required}"
case "${1:-}" in
  checks) cache="$HOME/.cache/tt-ci/target" ;;
  web)
    case "${RUNNER_NAME:-}" in
      ''|*[!a-zA-Z0-9_.-]*) printf 'runner name must be a path-safe slug\n' >&2; exit 1 ;;
    esac
    cache="$HOME/.cache/tt-ci/web/$RUNNER_NAME"
    ;;
  *) printf 'usage: prepare-ci-workspace.sh checks|web\n' >&2; exit 1 ;;
esac
mkdir -p "$cache" "$GITHUB_WORKSPACE/output/tmp"
chmod 700 "$GITHUB_WORKSPACE/output/tmp"
printf 'CARGO_TARGET_DIR=%s\n' "$cache" >> "$GITHUB_ENV"
printf 'TMPDIR=%s/output/tmp\n' "$GITHUB_WORKSPACE" >> "$GITHUB_ENV"
printf 'CARGO_BUILD_JOBS=1\nRUST_TEST_THREADS=1\n' >> "$GITHUB_ENV"
