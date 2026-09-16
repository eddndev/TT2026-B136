#!/usr/bin/env bash
# Exercise exact fact declarations in api-demo.sh's disposable database.
procedural_facts_demo_python() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    python3 -B "$REPO_ROOT/scripts/api-procedural-facts-demo.py" "$1"
}
procedural_facts_demo() { procedural_facts_demo_python capture; }
procedural_facts_demo_restored() { procedural_facts_demo_python restore; }
