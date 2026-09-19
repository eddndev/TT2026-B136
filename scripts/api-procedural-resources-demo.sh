#!/usr/bin/env bash
# Use exact fact fixtures and the shared disposable server and restore lifecycle.
procedural_resources_demo_python() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    python3 -B "$REPO_ROOT/scripts/api-procedural-resources-demo.py" "$1"
}
procedural_resources_demo() { procedural_resources_demo_python capture; }
procedural_resources_demo_restored() { procedural_resources_demo_python restore; }
