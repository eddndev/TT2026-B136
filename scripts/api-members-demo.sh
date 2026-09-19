#!/usr/bin/env bash
# Capture account access and assignment selectors in the disposable restore rehearsal.
member_demo_python() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    python3 -B "$REPO_ROOT/scripts/api-members-demo.py" "$1"
}
member_demo() { member_demo_python capture; }
member_demo_restored() { member_demo_python restore; }
