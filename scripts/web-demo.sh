#!/usr/bin/env bash
# Exercise the browser against isolated Rust, PostgreSQL, Redis, and local TSA services.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ "${1:-}" != "--with-backends" ]; then
  exec bash "$REPO_ROOT/scripts/test-backends.sh" bash "$0" --with-backends "$@"
fi
shift
: "${IDENTITY_TEST_DATABASE_URL:?disposable backends are required}"
: "${IDENTITY_TEST_REDIS_URL:?disposable backends are required}"
: "${TT_TEST_QPDF_LIBRARY:?native document validation library is required}"
export DOCUMENT_QPDF_LIBRARY="${DOCUMENT_QPDF_LIBRARY:-$TT_TEST_QPDF_LIBRARY}"
export TT_DEADLINE_REEVALUATION_ACCEPTANCE=1

for command in cargo curl jq openssl python3 node psql unzip; do
  command -v "$command" >/dev/null || {
    printf 'web-demo.sh: required command not found: %s\n' "$command" >&2
    exit 1
  }
done

WORK_DIR="$(mktemp -d)"
# Stop dotenv discovery before it can reach an ancestor checkout's settings.
: >"$WORK_DIR/.env"
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
CLI="$(cd "${CARGO_TARGET_DIR:-$REPO_ROOT/target}" && pwd)/debug/despacho-cli"
export DATABASE_URL="$IDENTITY_TEST_DATABASE_URL"
export REDIS_URL="$IDENTITY_TEST_REDIS_URL"
export PKI_CA_DIR="$WORK_DIR/ca"
export TSA_DIR="$WORK_DIR/tsa"
export KEK_BASE64
KEK_BASE64="$(openssl rand -base64 32)"
unset CINCEL_BASE_URL CINCEL_API_KEY RESEND_API_KEY ALERT_EMAIL_FROM ALERT_LOGIN_URL
TT_BROWSER_DATABASE_PASSWORD="$(openssl rand -hex 24)"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -v runtime_password="$TT_BROWSER_DATABASE_PASSWORD" >/dev/null <<'SQL'
SET password_encryption = 'scram-sha-256';
CREATE ROLE tt_browser LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT
  PASSWORD :'runtime_password';
SQL
"$CLI" database migrate --runtime-role tt_browser >/dev/null
DATABASE_URL="$(TT_BROWSER_DATABASE_PASSWORD="$TT_BROWSER_DATABASE_PASSWORD" node -e '
  const url = new URL(process.env.IDENTITY_TEST_DATABASE_URL);
  url.username = "tt_browser";
  url.password = process.env.TT_BROWSER_DATABASE_PASSWORD;
  process.stdout.write(url.href);
')"
export DATABASE_URL
"$CLI" pki --scripts-dir "$REPO_ROOT/pki" init-ca >/dev/null
"$CLI" pki --scripts-dir "$REPO_ROOT/pki" issue --cn 'Browser Demo' >/dev/null
"$CLI" pki --scripts-dir "$REPO_ROOT/pki" issue --cn 'Browser Participant' \
  --purpose participant-declaration >/dev/null
mkdir -m 700 "$WORK_DIR/client-credentials" "$WORK_DIR/server"
mv "$PKI_CA_DIR/private/browser-participant.key.pem" "$WORK_DIR/client-credentials/participant.key.pem"
bash "$REPO_ROOT/pki/issue-tsa-cert.sh" >/dev/null
"$CLI" pki --scripts-dir "$REPO_ROOT/pki" gen-crl >/dev/null
DATABASE_URL="$IDENTITY_TEST_DATABASE_URL" "$CLI" credential-trust publish \
  --root-cert "$PKI_CA_DIR/ca.crt.pem" --crl "$PKI_CA_DIR/crl/crl.pem" \
  --expected-revision 0 >/dev/null

(
  cd "$WORK_DIR/server"
  exec "$CLI" serve --bind 127.0.0.1:0 --data-dir "$WORK_DIR/data" \
  --deadline-page-limit 2 --deadline-poll-ms 50 \
  --signer-cert "$PKI_CA_DIR/certs/browser-demo.crt.pem" \
  --signer-key "$PKI_CA_DIR/private/browser-demo.key.pem" \
  --ca-cert "$PKI_CA_DIR/ca.crt.pem" --crl "$PKI_CA_DIR/crl/crl.pem" \
  --tsa-config "$REPO_ROOT/pki/tsa.cnf" --tsa-dir "$TSA_DIR"
) >"$WORK_DIR/server.log" 2>&1 &
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
export TT_LIVE_PARTICIPANT_CERTIFICATE="$PKI_CA_DIR/certs/browser-participant.crt.pem"
export TT_LIVE_PARTICIPANT_PRIVATE_KEY="$WORK_DIR/client-credentials/participant.key.pem"
umask 077
curl -fsS -X POST "$API_PROXY_TARGET/api/v1/auth/bootstrap" \
  -H 'Content-Type: application/json' \
  --data '{"email":"browser@example.com","password":"browser demonstration password"}' \
  | jq '{email: .user.email, password: "browser demonstration password", recoveryCodes: .recovery_codes}' \
  >"$TT_WEB_FIXTURES"
node "$REPO_ROOT/scripts/web-participant-fixtures.mjs"
node "$REPO_ROOT/scripts/web-case-administration-fixtures.mjs"
node "$REPO_ROOT/scripts/web-case-stage-fixtures.mjs"
node "$REPO_ROOT/scripts/web-hearing-fixtures.mjs"
export TT_WEB_PORT
TT_WEB_PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
cd "$REPO_ROOT/web"
node node_modules/@playwright/test/cli.js test -c playwright.live.config.mjs "$@"
