#!/usr/bin/env bash
# Settings for disposable CI data only; see docs/adr/0047-disposable-postgres-ci.md.
set -euo pipefail

psql "$IDENTITY_TEST_DATABASE_URL" -X -v ON_ERROR_STOP=1 \
  -c 'ALTER SYSTEM SET fsync = off' \
  -c 'ALTER SYSTEM SET synchronous_commit = off' \
  -c 'ALTER SYSTEM SET full_page_writes = off' \
  -c "ALTER SYSTEM SET checkpoint_timeout = '30min'" \
  -c "ALTER SYSTEM SET max_wal_size = '2GB'" \
  -c 'SELECT pg_reload_conf()'

for attempt in 1 2 3 4 5; do
  if bash scripts/tests/check_ci_postgres.sh; then
    exit 0
  fi
  sleep 1
done
printf 'disposable PostgreSQL did not apply CI settings\n' >&2
exit 1
