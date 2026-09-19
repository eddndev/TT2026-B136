#!/usr/bin/env bash
# Exercise the integrated inbox after the existing restore comparisons finish.
TT_DEADLINE_API_BASE_URL="$BASE_URL" TT_DEADLINE_API_TOKEN="$RECOVERY_TOKEN" \
  TT_DEADLINE_API_WORK_DIR="$WORK_DIR" \
  python3 -B "$REPO_ROOT/scripts/api-alerts-demo.py"
