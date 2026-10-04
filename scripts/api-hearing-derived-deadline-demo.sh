#!/usr/bin/env bash
# Add compound hearing result acceptance to the disposable restore lifecycle.
hearing_derived_deadline_demo() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    TT_DEADLINE_API_BASE_URL="$BASE_URL" TT_DEADLINE_API_TOKEN="$RECOVERY_TOKEN" \
    TT_DEADLINE_API_WORK_DIR="$WORK_DIR" \
    python3 -B "$REPO_ROOT/scripts/api-hearing-derived-deadline-demo.py" "$1"
  local case_id
  case_id="$(jq -er '.scenario.case_id' "$WORK_DIR/hearing-derived-deadline-api-state.json")"
  [ "$(psql "$2" -X -v ON_ERROR_STOP=1 -v case_id="$case_id" -At <<'SQL'
WITH origins AS (
  SELECT o.*, 'hrdc1:operation:' || o.operation_id::text || ':capture:' ||
    encode(o.capture_digest,'hex') AS marker
  FROM case_hearing_derived_deadline_origins o WHERE o.case_id=:'case_id'::uuid
)
SELECT count(*)=2 AND bool_and(COALESCE(
  a.action='hearing_derived_deadline.registered' AND a.resource=o.marker
  AND a.actor=o.actor_email AND
  (SELECT count(*) FROM audit_events repeated
   WHERE repeated.action='hearing_derived_deadline.registered'
     AND repeated.resource LIKE 'hrdc1:operation:'||o.operation_id::text||':capture:%')=1,
  FALSE))
FROM origins o LEFT JOIN audit_events a ON a.sequence=o.audit_sequence;
SQL
)" = t ]
  printf 'Compound origins and their unique creation audit entries verified after %s.\n' "$1"
}
