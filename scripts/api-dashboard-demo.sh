#!/usr/bin/env bash
# Compare authorized operational snapshots in the disposable restore rehearsal.
dashboard_demo_python() {
  TT_DASHBOARD_API_BASE_URL="$BASE_URL" TT_DASHBOARD_API_TOKEN="$RECOVERY_TOKEN" \
    TT_DASHBOARD_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-dashboard-demo.py" "$1"
}
dashboard_demo_capture() { dashboard_demo_python capture; }
dashboard_demo_restored() { dashboard_demo_python restore; }
