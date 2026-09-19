#!/usr/bin/env bash
# Capture the integrated inbox before backup and compare it after restoration.
alert_demo_python() {
  TT_DEADLINE_API_BASE_URL="$BASE_URL" TT_DEADLINE_API_TOKEN="$RECOVERY_TOKEN" \
    TT_DEADLINE_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-alerts-demo.py" "$1"
}
alert_demo() { alert_demo_python capture; }
alert_demo_restored() { alert_demo_python restore; }
