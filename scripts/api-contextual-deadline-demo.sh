#!/usr/bin/env bash
# Add contextual deadline acceptance to the existing disposable restore lifecycle.
contextual_deadline_demo() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    TT_DEADLINE_API_BASE_URL="$BASE_URL" TT_DEADLINE_API_TOKEN="$RECOVERY_TOKEN" \
    TT_DEADLINE_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-contextual-deadline-demo.py" "$1"
}
