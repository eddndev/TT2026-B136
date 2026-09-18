#!/usr/bin/env bash
# Exercise persistent deadlines in the disposable HTTP and restore cluster.
deadline_demo_python() {
  TT_DEADLINE_API_BASE_URL="$BASE_URL" TT_DEADLINE_API_TOKEN="$RECOVERY_TOKEN" \
    TT_DEADLINE_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-deadlines-demo.py" "$1"
}
deadline_demo() { deadline_demo_python capture; }
deadline_demo_restored() { deadline_demo_python restore; }
