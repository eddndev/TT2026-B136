#!/usr/bin/env bash
# Run tests with isolated PostgreSQL databases and a disposable Redis server.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for command in cargo initdb pg_ctl psql python3 redis-cli redis-server; do
  command -v "$command" >/dev/null || {
    printf 'test-backends.sh: required command not found: %s\n' "$command" >&2
    exit 1
  }
done

TEST_DIR="$(mktemp -d)"
TEST_DATABASE_USER="tt_backend_test_admin"
TEST_DATABASE_PASSWORD="$(python3 -c 'import secrets;print(secrets.token_hex(24))')"
POSTGRES_STARTED=false
REDIS_PID=""
free_port() {
  python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()'
}
PG_PORT="$(free_port)"
REDIS_PORT="$(free_port)"
while [ "$REDIS_PORT" = "$PG_PORT" ]; do REDIS_PORT="$(free_port)"; done

cleanup() {
  local status=$?
  if [ "$POSTGRES_STARTED" = true ]; then
    pg_ctl -D "$TEST_DIR/postgres" -m fast -w stop >/dev/null 2>&1 || true
  fi
  if [ -n "$REDIS_PID" ]; then
    kill "$REDIS_PID" 2>/dev/null || true
    wait "$REDIS_PID" 2>/dev/null || true
  fi
  rm -rf -- "$TEST_DIR"
  return "$status"
}
trap cleanup EXIT

printf '%s\n' "$TEST_DATABASE_PASSWORD" > "$TEST_DIR/postgres-password"
initdb -D "$TEST_DIR/postgres" --auth=scram-sha-256 --no-locale --encoding=UTF8 \
  --username="$TEST_DATABASE_USER" --pwfile="$TEST_DIR/postgres-password" >/dev/null
pg_ctl -D "$TEST_DIR/postgres" -l "$TEST_DIR/postgres.log" \
  -o "-p $PG_PORT -k $TEST_DIR -h 127.0.0.1" -w start >/dev/null
POSTGRES_STARTED=true
redis-server --port "$REDIS_PORT" --bind 127.0.0.1 --save "" \
  --appendonly no --daemonize no --dir "$TEST_DIR" >"$TEST_DIR/redis.log" 2>&1 &
REDIS_PID=$!
for _ in $(seq 1 50); do
  sleep 0.1
  kill -0 "$REDIS_PID" 2>/dev/null || { cat "$TEST_DIR/redis.log" >&2; exit 1; }
  if [ "$(redis-cli -p "$REDIS_PORT" ping 2>/dev/null || true)" = PONG ]; then break; fi
done

TEST_DATABASE_BASE="postgresql://$TEST_DATABASE_USER:$TEST_DATABASE_PASSWORD@127.0.0.1:$PG_PORT"
export IDENTITY_TEST_DATABASE_URL="$TEST_DATABASE_BASE/postgres"
export CASE_TEST_DATABASE_URL="$TEST_DATABASE_BASE/case_tests"
export DOCUMENT_TEST_DATABASE_URL="$TEST_DATABASE_BASE/document_tests"
export IDENTITY_TEST_REDIS_URL="redis://127.0.0.1:$REDIS_PORT/"
psql "$IDENTITY_TEST_DATABASE_URL" -v ON_ERROR_STOP=1 \
  -c 'CREATE DATABASE case_tests' -c 'CREATE DATABASE document_tests' >/dev/null
[ "$(redis-cli -p "$REDIS_PORT" ping)" = PONG ]

cd "$REPO_ROOT"
if [ "$#" -eq 0 ]; then
  cargo test --workspace
else
  "$@"
fi
