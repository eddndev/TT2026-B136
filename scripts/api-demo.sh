#!/usr/bin/env bash
# Automated end-to-end smoke test for the authenticated case and document API.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKI_SCRIPTS="$REPO_ROOT/pki"

for command in cargo curl initdb jq openssl pg_ctl pg_dump pg_restore psql python3 redis-cli redis-server rg unzip stdbuf; do
  command -v "$command" >/dev/null 2>&1 || {
    printf 'api-demo.sh: required command not found: %s\n' "$command" >&2
    exit 1
  }
done

if [ -z "${DOCUMENT_QPDF_LIBRARY:-}" ]; then
  if [ -n "${TT_TEST_QPDF_LIBRARY:-}" ]; then
    DOCUMENT_QPDF_LIBRARY="$TT_TEST_QPDF_LIBRARY"
  else
    DOCUMENT_QPDF_LIBRARY="$(bash "$REPO_ROOT/scripts/setup-document-formats.sh")"
  fi
fi
export DOCUMENT_QPDF_LIBRARY

cargo build --workspace --manifest-path "$REPO_ROOT/Cargo.toml"
CLI="$REPO_ROOT/target/debug/despacho-cli"
WORK_DIR="$(mktemp -d)"
SERVER_PID=""
SECOND_SERVER_PID=""
POSTGRES_STARTED="false"
REDIS_PID=""
PG_DATA="$WORK_DIR/postgres"
PG_PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
REDIS_PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"

cleanup() {
  local status=$?
  if [ "$status" -ne 0 ] && [ -f "${SERVER_LOG:-}" ]; then
    printf 'api-demo.sh: server log after failure\n' >&2
    cat "$SERVER_LOG" >&2
  fi
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if [ -n "$SECOND_SERVER_PID" ] && kill -0 "$SECOND_SERVER_PID" 2>/dev/null; then
    kill "$SECOND_SERVER_PID" 2>/dev/null || true
    wait "$SECOND_SERVER_PID" 2>/dev/null || true
  fi
  if [ "$POSTGRES_STARTED" = "true" ]; then
    pg_ctl -D "$PG_DATA" -m fast stop >/dev/null 2>&1 || true
  fi
  if [ -n "$REDIS_PID" ]; then
    kill "$REDIS_PID" 2>/dev/null || true
    wait "$REDIS_PID" 2>/dev/null || true
  fi
  if [ -d "$WORK_DIR" ] && [[ "$WORK_DIR" == /tmp/* ]]; then
    rm -rf -- "$WORK_DIR"
  fi
  return "$status"
}
trap cleanup EXIT

export PKI_CA_DIR="$WORK_DIR/pki-ca"
export TSA_DIR="$WORK_DIR/pki-tsa"
export KEK_BASE64
KEK_BASE64="$(openssl rand -base64 32)"
unset CINCEL_BASE_URL CINCEL_API_KEY
DATABASE_ADMIN_URL="postgresql://127.0.0.1:$PG_PORT/postgres"
export DATABASE_URL="$DATABASE_ADMIN_URL"
export REDIS_URL="redis://127.0.0.1:$REDIS_PORT/"

initdb -D "$PG_DATA" --auth=trust --no-locale --encoding=UTF8 >/dev/null
pg_ctl -D "$PG_DATA" -o "-p $PG_PORT -k $WORK_DIR" -w start >/dev/null
POSTGRES_STARTED="true"
psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -c \
  'CREATE ROLE tt_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT' >/dev/null
"$CLI" database migrate --runtime-role tt_runtime >/dev/null
export DATABASE_URL="postgresql://tt_runtime@127.0.0.1:$PG_PORT/postgres"
redis-server --port "$REDIS_PORT" --bind 127.0.0.1 --save "" \
  --appendonly no --daemonize no --dir "$WORK_DIR" >"$WORK_DIR/redis.log" 2>&1 &
REDIS_PID=$!
for _ in $(seq 1 50); do
  sleep 0.1
  kill -0 "$REDIS_PID" 2>/dev/null || { cat "$WORK_DIR/redis.log" >&2; exit 1; }
  if [ "$(redis-cli -p "$REDIS_PORT" ping 2>/dev/null || true)" = PONG ]; then break; fi
done
[ "$(redis-cli -p "$REDIS_PORT" ping)" = PONG ]

"$CLI" pki --scripts-dir "$PKI_SCRIPTS" init-ca >/dev/null
"$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue --cn "API Demo" >/dev/null
bash "$PKI_SCRIPTS/issue-tsa-cert.sh" >/dev/null
"$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl >/dev/null

CERT="$PKI_CA_DIR/certs/api-demo.crt.pem"
KEY="$PKI_CA_DIR/private/api-demo.key.pem"
CA="$PKI_CA_DIR/ca.crt.pem"
CRL="$PKI_CA_DIR/crl/crl.pem"
DATA_DIR="$WORK_DIR/runtime-data"
SERVER_LOG="$WORK_DIR/server.log"

RUST_LOG=warn stdbuf -oL -eL "$CLI" serve \
  --bind 127.0.0.1:0 \
  --data-dir "$DATA_DIR" \
  --signer-cert "$CERT" \
  --signer-key "$KEY" \
  --ca-cert "$CA" \
  --crl "$CRL" \
  --tsa-config "$PKI_SCRIPTS/tsa.cnf" \
  --tsa-dir "$TSA_DIR" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

SERVER_ADDRESS=""
for _ in $(seq 1 100); do
  SERVER_ADDRESS="$(sed -n 's/^listening on http:\/\///p' "$SERVER_LOG" | tail -n 1)"
  if [ -n "$SERVER_ADDRESS" ]; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$SERVER_LOG" >&2
    exit 1
  fi
  sleep 0.1
done
[ -n "$SERVER_ADDRESS" ] || {
  cat "$SERVER_LOG" >&2
  printf 'api-demo.sh: server did not report its address\n' >&2
  exit 1
}
BASE_URL="http://$SERVER_ADDRESS"

curl -fsS "$BASE_URL/healthz" | rg -x 'ok' >/dev/null
printf 'Expediente API local verificable.\n' >"$WORK_DIR/document.txt"

BOOTSTRAP="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/bootstrap" \
  -H 'Content-Type: application/json' \
  --data '{"email":"owner@example.com","password":"correct horse battery staple"}')"
OWNER_SECRET="$(jq -er '.totp_secret_base32' <<<"$BOOTSTRAP")"
OWNER_RECOVERY="$(jq -er '.recovery_codes[0]' <<<"$BOOTSTRAP")"

totp_code() {
  python3 - "$1" <<'PY'
import base64
import hashlib
import hmac
import struct
import sys
import time

secret = base64.b32decode(sys.argv[1])
counter = int(time.time()) // 30
digest = hmac.new(secret, struct.pack(">Q", counter), hashlib.sha1).digest()
offset = digest[-1] & 15
value = struct.unpack(">I", digest[offset:offset + 4])[0] & 0x7fffffff
print(f"{value % 1_000_000:06d}")
PY
}

OWNER_CHALLENGE="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  --data '{"email":"owner@example.com","password":"correct horse battery staple"}' \
  | jq -er '.challenge_token')"
OWNER_CODE="$(totp_code "$OWNER_SECRET")"
OWNER_TOKEN="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/mfa/totp" \
  -H 'Content-Type: application/json' \
  --data "{\"challenge_token\":\"$OWNER_CHALLENGE\",\"code\":\"$OWNER_CODE\"}" \
  | jq -er '.access_token')"

UNAUTHORIZED_STATUS="$(curl -sS -o /dev/null -w '%{http_code}' -X POST \
  "$BASE_URL/api/v1/cases/00000000-0000-0000-0000-000000000001/documents" -H 'X-Document-Name: document.txt' \
  --data-binary "@$WORK_DIR/document.txt")"
[ "$UNAUTHORIZED_STATUS" = "401" ]

PARALEGAL="$(curl -fsS -X POST "$BASE_URL/api/v1/users" \
  -H "Authorization: Bearer $OWNER_TOKEN" \
  -H 'Content-Type: application/json' \
  --data '{"email":"helper@example.com","password":"another safe password","role":"paralegal"}')"
PARALEGAL_SECRET="$(jq -er '.totp_secret_base32' <<<"$PARALEGAL")"
PARALEGAL_CHALLENGE="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  --data '{"email":"helper@example.com","password":"another safe password"}' \
  | jq -er '.challenge_token')"
PARALEGAL_CODE="$(totp_code "$PARALEGAL_SECRET")"
PARALEGAL_TOKEN="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/mfa/totp" \
  -H 'Content-Type: application/json' \
  --data "{\"challenge_token\":\"$PARALEGAL_CHALLENGE\",\"code\":\"$PARALEGAL_CODE\"}" \
  | jq -er '.access_token')"

# shellcheck source=scripts/api-case-demo.sh
source "$REPO_ROOT/scripts/api-case-demo.sh"

curl -fsS -X POST "$BASE_URL/api/v1/auth/logout" \
  -H "Authorization: Bearer $OWNER_TOKEN" >/dev/null
REVOKED_STATUS="$(curl -sS -o /dev/null -w '%{http_code}' \
  "$BASE_URL/api/v1/auth/me" -H "Authorization: Bearer $OWNER_TOKEN")"
[ "$REVOKED_STATUS" = "401" ]

RECOVERY_CHALLENGE="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  --data '{"email":"owner@example.com","password":"correct horse battery staple"}' \
  | jq -er '.challenge_token')"
RECOVERY_TOKEN="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/mfa/recovery" \
  -H 'Content-Type: application/json' \
  --data "{\"challenge_token\":\"$RECOVERY_CHALLENGE\",\"code\":\"$OWNER_RECOVERY\"}" \
  | jq -er '.access_token')"

REPLAY_CHALLENGE="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  --data '{"email":"owner@example.com","password":"correct horse battery staple"}' \
  | jq -er '.challenge_token')"
REPLAY_STATUS="$(curl -sS -o /dev/null -w '%{http_code}' -X POST \
  "$BASE_URL/api/v1/auth/mfa/recovery" -H 'Content-Type: application/json' \
  --data "{\"challenge_token\":\"$REPLAY_CHALLENGE\",\"code\":\"$OWNER_RECOVERY\"}")"
[ "$REPLAY_STATUS" = "401" ]

AUDIT="$(curl -fsS "$BASE_URL/api/v1/audit/verify" \
  -H "Authorization: Bearer $RECOVERY_TOKEN")"
jq -e '.valid == true and .entries >= 10' <<<"$AUDIT" >/dev/null

psql "$DATABASE_URL" -Atc 'SELECT password_hash FROM users' \
  | rg -F 'correct horse battery staple' >/dev/null && {
    printf 'api-demo.sh: plaintext password leaked into PostgreSQL\n' >&2
    exit 1
  }
if redis-cli -p "$REDIS_PORT" keys '*' | rg -F "$RECOVERY_TOKEN" >/dev/null; then
  printf 'api-demo.sh: raw session token leaked into Redis keys\n' >&2
  exit 1
fi

# shellcheck source=scripts/api-concurrency-demo.sh
source "$REPO_ROOT/scripts/api-concurrency-demo.sh"

# shellcheck source=scripts/api-hearings-demo.sh
source "$REPO_ROOT/scripts/api-hearings-demo.sh"

# shellcheck source=scripts/api-migration-demo.sh
source "$REPO_ROOT/scripts/api-migration-demo.sh"

printf 'Authenticated API demo passed: %s\n' "$DOCUMENT_ID"
