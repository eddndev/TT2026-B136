#!/usr/bin/env bash
# Exercise the browser against isolated Rust, PostgreSQL, Redis, and local TSA services.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ "${1:-}" != "--with-backends" ]; then
  exec bash "$REPO_ROOT/scripts/test-backends.sh" bash "$0" --with-backends
fi
: "${IDENTITY_TEST_DATABASE_URL:?disposable backends are required}"
: "${IDENTITY_TEST_REDIS_URL:?disposable backends are required}"

for command in cargo curl jq openssl python3 node psql unzip; do
  command -v "$command" >/dev/null || {
    printf 'web-demo.sh: required command not found: %s\n' "$command" >&2
    exit 1
  }
done

WORK_DIR="$(mktemp -d)"
SERVER_PID=""
cleanup() {
  local status=$?
  if [ "$status" -ne 0 ] && [ -f "$WORK_DIR/server.log" ]; then
    cat "$WORK_DIR/server.log" >&2
  fi
  if [ -n "$SERVER_PID" ]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf -- "$WORK_DIR"
  return "$status"
}
trap cleanup EXIT

cargo build --workspace --manifest-path "$REPO_ROOT/Cargo.toml"
CLI="$REPO_ROOT/target/debug/despacho-cli"
export DATABASE_URL="$IDENTITY_TEST_DATABASE_URL"
export REDIS_URL="$IDENTITY_TEST_REDIS_URL"
export PKI_CA_DIR="$WORK_DIR/ca"
export TSA_DIR="$WORK_DIR/tsa"
export KEK_BASE64
KEK_BASE64="$(openssl rand -base64 32)"
unset CINCEL_BASE_URL CINCEL_API_KEY
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
  'CREATE ROLE tt_browser LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT' >/dev/null
"$CLI" database migrate --runtime-role tt_browser >/dev/null
export DATABASE_URL="${IDENTITY_TEST_DATABASE_URL/postgresql:\/\//postgresql:\/\/tt_browser@}"
"$CLI" pki --scripts-dir "$REPO_ROOT/pki" init-ca >/dev/null
"$CLI" pki --scripts-dir "$REPO_ROOT/pki" issue --cn 'Browser Demo' >/dev/null
bash "$REPO_ROOT/pki/issue-tsa-cert.sh" >/dev/null
"$CLI" pki --scripts-dir "$REPO_ROOT/pki" gen-crl >/dev/null

"$CLI" serve --bind 127.0.0.1:0 --data-dir "$WORK_DIR/data" \
  --signer-cert "$PKI_CA_DIR/certs/browser-demo.crt.pem" \
  --signer-key "$PKI_CA_DIR/private/browser-demo.key.pem" \
  --ca-cert "$PKI_CA_DIR/ca.crt.pem" --crl "$PKI_CA_DIR/crl/crl.pem" \
  --tsa-config "$REPO_ROOT/pki/tsa.cnf" --tsa-dir "$TSA_DIR" \
  >"$WORK_DIR/server.log" 2>&1 &
SERVER_PID=$!
SERVER_ADDRESS=""
for _ in $(seq 1 100); do
  SERVER_ADDRESS="$(sed -n 's/^listening on http:\/\///p' "$WORK_DIR/server.log" | tail -n 1)"
  if [ -n "$SERVER_ADDRESS" ]; then break; fi
  kill -0 "$SERVER_PID" 2>/dev/null || exit 1
  sleep 0.1
done
[ -n "$SERVER_ADDRESS" ]
export API_PROXY_TARGET="http://$SERVER_ADDRESS"
export TT_WEB_FIXTURES="$WORK_DIR/browser-fixtures.json"
umask 077
curl -fsS -X POST "$API_PROXY_TARGET/api/v1/auth/bootstrap" \
  -H 'Content-Type: application/json' \
  --data '{"email":"browser@example.com","password":"browser demonstration password"}' \
  | jq '{email: .user.email, password: "browser demonstration password", recoveryCodes: .recovery_codes}' \
  >"$TT_WEB_FIXTURES"
export TT_WEB_PORT
TT_WEB_PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
cd "$REPO_ROOT/web"
node node_modules/@playwright/test/cli.js test -c playwright.live.config.mjs
