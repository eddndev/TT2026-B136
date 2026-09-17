#!/usr/bin/env bash
# Exercise global and case profile receipts in the disposable HTTP cluster.
profile_demo_python() {
  TT_PROFILE_API_BASE_URL="$BASE_URL" TT_PROFILE_API_TOKEN="$RECOVERY_TOKEN" \
    TT_PROFILE_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-deadline-profiles-demo.py" "$1"
}
profile_demo() { profile_demo_python capture; }
profile_demo_restored() { profile_demo_python restore; }
