#!/usr/bin/env bash
# Reuse existing disposable hearing and deadline fixtures without new mutations.
agenda_demo() {
  TT_DEADLINE_API_BASE_URL="$BASE_URL" TT_DEADLINE_API_TOKEN="$RECOVERY_TOKEN" \
    TT_DEADLINE_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-agenda-demo.py"
}
