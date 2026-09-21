#!/usr/bin/env bash
# Content acceptance shares api-demo.sh's disposable server and restore lifecycle.
document_content_demo_python() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    TT_CONTENT_ADMIN_URL="${2:-}" TT_CONTENT_PG_DATA="$PG_DATA" \
    TT_CONTENT_PG_PORT="$PG_PORT" \
    python3 -B "$REPO_ROOT/scripts/api-document-content-demo.py" "$1"
}
document_content_demo() { document_content_demo_python capture "$1"; }
document_content_demo_restored() { document_content_demo_python restore; }
