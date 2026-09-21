#!/usr/bin/env bash
# Reuse the shared server, historical targets and full database restore lifecycle.
resource_activities_demo_python() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    python3 -B "$REPO_ROOT/scripts/api-resource-activities-demo.py" "$1"
}
resource_activities_demo() { resource_activities_demo_python capture; }
resource_activities_demo_restored() { resource_activities_demo_python restore; }
