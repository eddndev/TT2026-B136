#!/usr/bin/env bash
# Restoration changes the database boundary and invalidates prior authentication.
identity_restore_run() {
  TT_RESTORE_WORK="$WORK_DIR" TT_RESTORE_REDIS_PORT="$REDIS_PORT" \
    TT_RESTORE_REDIS_PID="$REDIS_PID" TT_RESTORE_SERVER_PID="${SERVER_PID:-}" \
    TT_RESTORE_BASE="${BASE_URL:-}" TT_RESTORE_OLD_OWNER="$RECOVERY_TOKEN" \
    TT_RESTORE_OLD_HELPER="$PARALEGAL_TOKEN" TT_RESTORE_OWNER_SECRET="$OWNER_SECRET" \
    TT_RESTORE_HELPER_SECRET="$PARALEGAL_SECRET" \
    python3 -B "$REPO_ROOT/scripts/api_identity_restore.py" "$1"
}
identity_restore_login() {
  identity_restore_run login
  RECOVERY_TOKEN="$(jq -er '.owner' "$WORK_DIR/restored-identity.json")"
  PARALEGAL_TOKEN="$(jq -er '.helper' "$WORK_DIR/restored-identity.json")"
}
