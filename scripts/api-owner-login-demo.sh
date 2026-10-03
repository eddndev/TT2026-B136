#!/usr/bin/env bash
# Owner login shares the existing disposable database, CA, Redis and restore.

OWNER_LOGIN_DEMO_ARGS=(
  --owner-login-enabled
  --owner-login-start-global-max 64 --owner-login-start-global-window-seconds 3600
  --owner-login-start-owner-binding-max 64 --owner-login-start-owner-binding-window-seconds 3600
  --owner-login-proof-global-max 64 --owner-login-proof-global-window-seconds 3600
  --owner-login-proof-token-max 8 --owner-login-proof-token-window-seconds 3600
  --owner-login-redis-connect-ms 1000 --owner-login-redis-io-ms 1000
)

owner_login_demo_python() {
  TT_OWNER_CERT_BASE="$BASE_URL" TT_OWNER_CERT_TOKEN="$RECOVERY_TOKEN" \
    TT_OWNER_CERT_WORK="$WORK_DIR/owner-login" TT_OWNER_CERT_DATABASE="$2" \
    TT_OWNER_CERT_CERTIFICATE="$PKI_CA_DIR/certs/api-owner-login.crt.pem" \
    TT_OWNER_CERT_KEY="$WORK_DIR/owner-login/owner-client/private.key.pem" \
    TT_OWNER_LOGIN_WORK="$WORK_DIR" TT_OWNER_LOGIN_REDIS_PORT="$REDIS_PORT" \
    TT_OWNER_LOGIN_REDIS_PID="$REDIS_PID" TT_OWNER_LOGIN_SERVER_PID="${SERVER_PID:-}" \
    python3 -B "$REPO_ROOT/scripts/api_owner_login_demo.py" "$1"
}

owner_login_demo_prepare() {
  mkdir -m 700 "$WORK_DIR/owner-login" "$WORK_DIR/owner-login/owner-client"
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue --cn 'API Owner Login' >/dev/null
  mv "$PKI_CA_DIR/private/api-owner-login.key.pem" "$WORK_DIR/owner-login/owner-client/private.key.pem"
  chmod 600 "$WORK_DIR/owner-login/owner-client/private.key.pem"
  owner_login_demo_python prepare "$1"
}
