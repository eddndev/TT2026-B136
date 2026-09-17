#!/usr/bin/env bash
# Exercise hearing history inside api-demo.sh's disposable database and restore.

hearing_demo_python() {
  TT_HEARING_API_BASE_URL="$BASE_URL" TT_HEARING_API_TOKEN="$RECOVERY_TOKEN" \
    TT_HEARING_API_WORK_DIR="$WORK_DIR" TT_HEARING_API_REPO="$REPO_ROOT" \
    python3 "$REPO_ROOT/scripts/api-hearings-demo.py" "$1"
}

hearing_demo() {
  hearing_demo_python capture
}

hearing_demo_restored() {
  hearing_demo_python restore
}
