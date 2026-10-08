#!/usr/bin/env bash
# Reuse the disposable API campaign for precautionary restart and restore acceptance.
precautionary_demo() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    python3 -B "$REPO_ROOT/scripts/api-precautionary-demo.py" "$1"
}
