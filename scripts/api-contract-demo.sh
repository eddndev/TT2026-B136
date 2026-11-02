#!/usr/bin/env bash
# Bilateral contract signing end to end against disposable local services:
# docs/adr/0086-bilateral-contract-signing.md. PostgreSQL and Redis listen
# only on Unix sockets. The server and the HTTP client run in a private
# network namespace whose only interface is loopback, with the e-mail
# provider host pinned to loopback, so invitation and notice e-mail is
# attempted but never leaves this machine. The client key is generated
# here with OpenSSL; the server never receives it.

set -Eeuo pipefail
trap 'printf "api-contract-demo.sh: failure at %s:%s (status %s)\n" "${BASH_SOURCE[0]}" "$LINENO" "$?" >&2' ERR

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKI_SCRIPTS="$REPO_ROOT/pki"

if [ "${1:-}" = isolated ]; then
  # Runs inside the namespace: loopback up and the provider host pinned.
  printf '127.0.0.1 localhost api.resend.com\n' >"$WORK_DIR/hosts"
  printf 'hosts: files\n' >"$WORK_DIR/nsswitch.conf"
  mount --bind "$WORK_DIR/hosts" /etc/hosts
  mount --bind "$WORK_DIR/nsswitch.conf" /etc/nsswitch.conf
  ip link set lo up
  SERVER_PID=""
  stop_server() {
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    SERVER_PID=""
  }
  trap 'status=$?; [ "$status" -ne 0 ] && [ -f "$WORK_DIR/server.log" ] && cat "$WORK_DIR/server.log" >&2; stop_server; exit "$status"' EXIT
  start_server() {
    : >"$WORK_DIR/server.log"
    (
      cd "$WORK_DIR"
      DATABASE_URL="$1" RUST_LOG=warn exec "$CLI" serve --bind 127.0.0.1:0 \
        --data-dir "$WORK_DIR/runtime-data" --signer-cert "$CERT" --signer-key "$KEY" \
        --ca-cert "$CA" --crl "$CRL" --tsa-config "$PKI_SCRIPTS/tsa.cnf" --tsa-dir "$TSA_DIR"
    ) >>"$WORK_DIR/server.log" 2>&1 &
    SERVER_PID=$!
    BASE_URL=""
    for _ in $(seq 1 600); do
      BASE_URL="$(sed -n 's/^listening on //p' "$WORK_DIR/server.log" | tail -n 1)"
      [ -n "$BASE_URL" ] && break
      kill -0 "$SERVER_PID" 2>/dev/null || exit 1
      sleep 0.1
    done
    [ -n "$BASE_URL" ]
    export BASE_URL
  }
  start_server "$RUNTIME_URL"
  python3 -B "$REPO_ROOT/scripts/api-contract-demo.py" capture
  stop_server
  printf 'Contract restore: dumping and restoring the database.\n'
  pg_dump "$ADMIN_DSN" --format=custom --file="$WORK_DIR/contracts.dump"
  psql "$ADMIN_DSN" -v ON_ERROR_STOP=1 -qc 'CREATE DATABASE restored'
  pg_restore --exit-on-error --dbname="$RESTORED_DSN" "$WORK_DIR/contracts.dump"
  start_server "$RESTORED_URL"
  python3 -B "$REPO_ROOT/scripts/api-contract-demo.py" restore
  stop_server
  exit 0
fi

for command in cargo initdb ip jq mount openssl pg_ctl pg_dump pg_restore psql python3 redis-cli redis-server unshare unzip; do
  command -v "$command" >/dev/null 2>&1 || {
    printf 'api-contract-demo.sh: required command not found: %s\n' "$command" >&2
    exit 1
  }
done
python3 -c 'import cryptography' 2>/dev/null || {
  printf 'api-contract-demo.sh: python3 cryptography is required\n' >&2
  exit 1
}
if [ -z "${DOCUMENT_QPDF_LIBRARY:-}" ]; then
  DOCUMENT_QPDF_LIBRARY="${TT_TEST_QPDF_LIBRARY:-$(bash "$REPO_ROOT/scripts/setup-document-formats.sh")}"
fi
export DOCUMENT_QPDF_LIBRARY

cargo build --workspace --manifest-path "$REPO_ROOT/Cargo.toml"
cargo build -p evidence-verify --example validate --manifest-path "$REPO_ROOT/Cargo.toml"
TARGET="$(cd "${CARGO_TARGET_DIR:-$REPO_ROOT/target}" && pwd)/debug"
export CLI="$TARGET/despacho-cli" VALIDATE="$TARGET/examples/validate"
WORK_DIR="$(mktemp -d)"
: >"$WORK_DIR/.env"
PG_STARTED=false
REDIS_PID=""
cleanup() {
  local status=$?
  if [ -n "$REDIS_PID" ]; then kill "$REDIS_PID" 2>/dev/null || true; wait "$REDIS_PID" 2>/dev/null || true; fi
  if [ "$PG_STARTED" = true ]; then pg_ctl -D "$WORK_DIR/postgres" -m fast stop >/dev/null 2>&1 || true; fi
  if [[ "$WORK_DIR" == "${TMPDIR:-/tmp}"/tmp.* ]]; then rm -rf -- "$WORK_DIR"; fi
  return "$status"
}
trap cleanup EXIT

PG_PORT=5432
initdb -D "$WORK_DIR/postgres" --auth=trust --no-locale --encoding=UTF8 -U tt_admin >/dev/null
pg_ctl -D "$WORK_DIR/postgres" -l "$WORK_DIR/postgres.log" \
  -o "-c listen_addresses='' -k $WORK_DIR -p $PG_PORT" -w start >/dev/null
PG_STARTED=true
redis-server --port 0 --unixsocket "$WORK_DIR/redis.sock" --unixsocketperm 700 --save "" \
  --appendonly no --daemonize no --dir "$WORK_DIR" >"$WORK_DIR/redis.log" 2>&1 &
REDIS_PID=$!
for _ in $(seq 1 50); do
  sleep 0.1
  [ "$(redis-cli -s "$WORK_DIR/redis.sock" ping 2>/dev/null || true)" = PONG ] && break
done
[ "$(redis-cli -s "$WORK_DIR/redis.sock" ping)" = PONG ]

SOCKET="$(python3 -c 'import sys,urllib.parse;print(urllib.parse.quote(sys.argv[1],safe=""))' "$WORK_DIR")"
export ADMIN_DSN="host=$WORK_DIR port=$PG_PORT user=tt_admin dbname=postgres"
export RESTORED_DSN="host=$WORK_DIR port=$PG_PORT user=tt_admin dbname=restored"
export RUNTIME_URL="postgresql://tt_runtime@$SOCKET:$PG_PORT/postgres"
export RESTORED_URL="postgresql://tt_runtime@$SOCKET:$PG_PORT/restored"
psql "$ADMIN_DSN" -v ON_ERROR_STOP=1 -qc \
  'CREATE ROLE tt_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT'
DATABASE_URL="postgresql://tt_admin@$SOCKET:$PG_PORT/postgres" "$CLI" database migrate \
  --runtime-role tt_runtime >/dev/null

export PKI_CA_DIR="$WORK_DIR/pki-ca" TSA_DIR="$WORK_DIR/pki-tsa" PKI_SCRIPTS
export KEK_BASE64
KEK_BASE64="$(openssl rand -base64 32)"
"$CLI" pki --scripts-dir "$PKI_SCRIPTS" init-ca >/dev/null
"$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue --cn "Contract Demo Seal" >/dev/null
bash "$PKI_SCRIPTS/issue-tsa-cert.sh" >/dev/null
export CA="$PKI_CA_DIR/ca.crt.pem" CRL="$PKI_CA_DIR/crl/crl.pem"
export CERT="$PKI_CA_DIR/certs/contract-demo-seal.crt.pem"
export KEY="$PKI_CA_DIR/private/contract-demo-seal.key.pem"
# Each party generates its own key and CSR; the CA only receives the CSR.
for party in client firm; do
  mkdir -m 700 "$WORK_DIR/$party"
  (
    umask 077
    openssl req -new -newkey rsa:3072 -nodes -sha256 \
      -subj "/C=MX/O=Despacho Juridico Demo/CN=Contract Demo ${party^}" \
      -keyout "$WORK_DIR/$party/private.key.pem" -out "$WORK_DIR/$party/public.csr.pem" >/dev/null 2>&1
  )
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue-request \
    --csr "$WORK_DIR/$party/public.csr.pem" --name "contract-$party" >/dev/null
  test ! -e "$PKI_CA_DIR/private/contract-$party.key.pem"
done
"$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl >/dev/null
DATABASE_URL="postgresql://tt_admin@$SOCKET:$PG_PORT/postgres" "$CLI" credential-trust publish \
  --root-cert "$CA" --crl "$CRL" --expected-revision 0 >/dev/null

# Invitations and notice e-mail are enabled; the provider is unreachable.
export REDIS_URL="redis+unix://$WORK_DIR/redis.sock" WORK_DIR REPO_ROOT
export TT_INVITATIONS_ENABLED=true INVITATION_EMAIL_FROM=avisos@example.test
export INVITATION_PUBLIC_URL=https://qadra.example.test/ RESEND_API_KEY=re_disposable_demo_key
export TT_CONTRACT_EMAIL_ENABLED=true REPORT_EMAIL_FROM=avisos@example.test
export REPORT_PUBLIC_URL=https://qadra.example.test/
unset TT_REPORT_EMAIL_ENABLED CINCEL_BASE_URL CINCEL_API_KEY ALERT_EMAIL_FROM ALERT_LOGIN_URL
unshare --map-root-user --net --mount bash "$REPO_ROOT/scripts/api-contract-demo.sh" isolated
printf 'Bilateral contract API demo passed.\n'
