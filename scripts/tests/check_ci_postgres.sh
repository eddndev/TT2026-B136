#!/usr/bin/env bash
# Verify that the disposable PostgreSQL service uses non-durable CI settings.
set -euo pipefail

settings=$(psql "$IDENTITY_TEST_DATABASE_URL" -X -At -v ON_ERROR_STOP=1 \
  -c "SELECT current_setting('fsync') || '|' ||
             current_setting('synchronous_commit') || '|' ||
             current_setting('full_page_writes')")
test "$settings" = 'off|off|off' || {
  printf 'unexpected disposable PostgreSQL settings: %s\n' "$settings" >&2
  exit 1
}
