#!/usr/bin/env bash
# Exercise global calendar receipts in api-demo.sh's disposable cluster.
calendar_demo_python() {
  TT_CALENDAR_API_BASE_URL="$BASE_URL" TT_CALENDAR_API_TOKEN="$RECOVERY_TOKEN" \
    TT_CALENDAR_API_WORK_DIR="$WORK_DIR" TT_CALENDAR_API_REPO="$REPO_ROOT" \
    python3 -B "$REPO_ROOT/scripts/api-judicial-calendars-demo.py" "$1"
}
calendar_demo() { calendar_demo_python capture; }
calendar_demo_restored() { calendar_demo_python restore; }
