#!/usr/bin/env bash
# Rehearse encrypted legacy cutover and backup restoration in api-demo.sh's cluster.

# shellcheck source=scripts/api-version-demo.sh
source "$REPO_ROOT/scripts/api-version-demo.sh"
# shellcheck source=scripts/api-metadata-demo.sh
source "$REPO_ROOT/scripts/api-metadata-demo.sh"
# shellcheck source=scripts/api-participant-demo.sh
source "$REPO_ROOT/scripts/api-participant-demo.sh"
# shellcheck source=scripts/api-case-administration-demo.sh
source "$REPO_ROOT/scripts/api-case-administration-demo.sh"
# shellcheck source=scripts/api-case-stage-demo.sh
source "$REPO_ROOT/scripts/api-case-stage-demo.sh"
# shellcheck source=scripts/api-typed-participant-demo.sh
source "$REPO_ROOT/scripts/api-typed-participant-demo.sh"

migration_demo_stop() {
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID"
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  SERVER_PID=""
}

migration_demo_start() {
  local url="$1" directory="$2" label="$3" address=""
  export DATABASE_URL="$url"
  SERVER_LOG="$WORK_DIR/$label-server.log"
  RUST_LOG=warn stdbuf -oL -eL "$CLI" serve --bind 127.0.0.1:0 \
    --data-dir "$directory" --signer-cert "$CERT" --signer-key "$KEY" \
    --ca-cert "$CA" --crl "$CRL" --tsa-config "$PKI_SCRIPTS/tsa.cnf" \
    --tsa-dir "$TSA_DIR" >"$SERVER_LOG" 2>&1 &
  SERVER_PID=$!
  for _ in $(seq 1 100); do
    address="$(sed -n 's/^listening on http:\/\///p' "$SERVER_LOG" | tail -n 1)"
    if [ -n "$address" ]; then break; fi
    kill -0 "$SERVER_PID" 2>/dev/null || { cat "$SERVER_LOG" >&2; return 1; }
    sleep 0.1
  done
  [ -n "$address" ] || { cat "$SERVER_LOG" >&2; return 1; }
  BASE_URL="http://$address"
  curl -fsS "$BASE_URL/healthz" | rg -x 'ok' >/dev/null
}

migration_demo_state() {
  psql "$1" -v ON_ERROR_STOP=1 -Atc \
    "SELECT jsonb_build_object(
      'documents',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d),
      'series',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM document_series s),
      'metadata',(SELECT jsonb_agg(to_jsonb(m) ORDER BY document_id,metadata_revision)
        FROM document_metadata_revisions m),
      'participants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM case_participants p),
      'participant_revisions',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM case_participant_revisions p),
      'subjects',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM case_subjects s),
      'subject_revisions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY subject_id,revision)
        FROM case_subject_revisions s),
      'typed_participants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM case_participant_typed_revisions p),
      'subject_reviews',(SELECT jsonb_agg(to_jsonb(s) ORDER BY subject_id,revision)
        FROM subject_identity_reviews s),
      'participant_reviews',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM participant_identity_reviews p),
      'participant_credentials',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM participant_credential_evidence p),
      'credential_trust',(SELECT jsonb_agg(to_jsonb(t) ORDER BY deployment_id,revision)
        FROM participant_credential_trust_revisions t),
      'credential_authority',(SELECT jsonb_agg(to_jsonb(a) ORDER BY deployment_id)
        FROM participant_credential_authority a),
      'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a),
      'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY fingerprint) FROM migration_receipts r),
      'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
      'cases',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM cases c),
      'case_administration',(SELECT jsonb_agg(to_jsonb(c) ORDER BY case_id,revision)
        FROM case_administration_revisions c),
      'initial_stages',(SELECT jsonb_agg(to_jsonb(s) ORDER BY case_id)
        FROM case_initial_stage_registrations s),
      'stage_revisions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY case_id,revision)
        FROM case_stage_revisions s),
      'hearings',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_hearings h),
      'hearing_revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY hearing_id,revision)
        FROM case_hearing_revisions h),
      'hearing_results',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_hearing_results h),
      'hearing_result_revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY result_id,revision)
        FROM case_hearing_result_revisions h),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id,user_id) FROM case_memberships m)
    )" | jq -Sc .
}

migration_demo_export() {
  local case_id="$1" destination="$2"
  curl -fsS -X POST "$BASE_URL/api/v1/cases/$case_id/documents/$DOCUMENT_ID/versions/1/verify" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" \
    | jq -e '.verdict == "valid" and .integrity.status == "passed"
      and .signature.status == "passed" and .certificate.status == "passed"
      and .timestamp.status == "passed"' >/dev/null
  curl -fsS "$BASE_URL/api/v1/cases/$case_id/documents/$DOCUMENT_ID/versions/1/evidence" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" -o "$destination.zip"
  cmp "$WORK_DIR/owner-evidence.zip" "$destination.zip"
  unzip -t "$destination.zip" >/dev/null
  unzip -q "$destination.zip" -d "$destination"
  cmp "$WORK_DIR/document.txt" "$destination/document.txt"
  (
    cd "$destination"
    openssl x509 -in certificado.pem -pubkey -noout -out signer.pub.pem
    openssl dgst -sha256 -verify signer.pub.pem -signature document.txt.sig document.txt >/dev/null
    cat ca.pem crl.pem >ca-and-crl.pem
    openssl verify -crl_check -CAfile ca-and-crl.pem certificado.pem >/dev/null
    openssl ts -verify -data document.txt -in document.txt.tsr -CAfile tsa-chain.pem >/dev/null
  )
  curl -fsS "$BASE_URL/api/v1/audit/verify" -H "Authorization: Bearer $RECOVERY_TOKEN" \
    | jq -e '.valid == true' >/dev/null
}

migration_demo() {
  local legacy_dir="$WORK_DIR/legacy-snapshot" imported_url restored_url
  local document_count sealed_count audit_count case_id import_report runtime_url
  imported_url="postgresql://127.0.0.1:$PG_PORT/imported"
  restored_url="postgresql://127.0.0.1:$PG_PORT/restored"
  migration_demo_stop
  mkdir -p "$legacy_dir/documents"

  # A legacy file has one snapshot per UUID; do not silently overwrite history.
  [ "$(psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -Atc \
    'SELECT COUNT(*) FROM (SELECT id FROM documents GROUP BY id HAVING COUNT(*) <> 1) duplicates')" -eq 0 ]

  # Reconstruct the versioned JSON format from actual encrypted, sealed records.
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -Atc \
    "SELECT jsonb_build_object('id',id,'version',version,'name',name,
      'digest',encode(digest,'hex'),
      'vault_base64',replace(encode(vault,'base64'),E'\n',''),'evidence',evidence)
      FROM documents ORDER BY id" >"$WORK_DIR/legacy-documents.jsonl"
  python3 - "$WORK_DIR/legacy-documents.jsonl" "$legacy_dir/documents" <<'PY'
import json
import pathlib
import sys
import uuid

for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    document = json.loads(line)
    document_id = str(uuid.UUID(document["id"]))
    target = pathlib.Path(sys.argv[2]) / (document_id + ".json")
    target.write_text(json.dumps(document, separators=(",", ":")) + "\n")
PY
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -Atc \
    "SELECT jsonb_build_object('sequence',sequence,'timestamp',timestamp,'actor',actor,
      'action',action,'resource',resource,'chain',encode(chain,'hex'))
      FROM audit_events ORDER BY sequence" >"$legacy_dir/audit.jsonl"
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -Atc \
    "SELECT jsonb_build_object('documents',jsonb_agg(jsonb_build_object(
      'document_id',id,'case_id',case_id) ORDER BY id)) FROM documents" \
    >"$legacy_dir/mapping.json"
  document_count="$(psql "$DATABASE_ADMIN_URL" -Atc 'SELECT COUNT(*) FROM documents')"
  audit_count="$(psql "$DATABASE_ADMIN_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  case_id="$(psql "$DATABASE_ADMIN_URL" -Atc \
    "SELECT case_id FROM documents WHERE id='$DOCUMENT_ID'")"
  [ "$document_count" -ge 3 ]
  sealed_count="$(psql "$DATABASE_ADMIN_URL" -Atc 'SELECT COUNT(*) FROM documents WHERE evidence IS NOT NULL')"
  [ "$sealed_count" -ge 2 ]
  # This is a legacy-format fixture, not a full migration of current case history.
  # Copy known original root facts into a fresh administrative baseline. Source
  # API roots, their required R1 revisions and audit events remain untouched.
  # The full backup below separately proves restoration of every current table.
  pg_dump "$DATABASE_ADMIN_URL" --data-only --no-owner --no-privileges \
    -t users >"$WORK_DIR/identity-users.sql"
  pg_dump "$DATABASE_ADMIN_URL" --data-only --no-owner --no-privileges \
    -t case_memberships >"$WORK_DIR/case-memberships.sql"
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -c \
    "COPY (SELECT id,title,reference,created_by,created_at,
      NULL::BIGINT AS required_initial_revision FROM cases ORDER BY id)
      TO STDOUT WITH (FORMAT CSV, HEADER)" >"$WORK_DIR/case-baselines.csv"
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -c 'CREATE DATABASE imported' >/dev/null
  DATABASE_URL="$imported_url" "$CLI" database migrate --runtime-role tt_runtime >/dev/null
  psql "$imported_url" -v ON_ERROR_STOP=1 -f "$WORK_DIR/identity-users.sql" >/dev/null
  psql "$imported_url" -v ON_ERROR_STOP=1 -c \
    '\copy cases(id,title,reference,created_by,created_at,required_initial_revision) FROM STDIN WITH CSV HEADER' \
    <"$WORK_DIR/case-baselines.csv" >/dev/null
  psql "$imported_url" -v ON_ERROR_STOP=1 -f "$WORK_DIR/case-memberships.sql" >/dev/null
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM cases WHERE required_initial_revision IS NOT NULL')" -eq 0 ]
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM case_administration_revisions')" -eq 0 ]
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM case_initial_stage_registrations')" -eq 0 ]

  DATABASE_URL="$imported_url" "$CLI" --json database import \
    --data-dir "$legacy_dir" --mapping "$legacy_dir/mapping.json" >"$WORK_DIR/import-inspection.json"
  jq -e --argjson documents "$document_count" --argjson entries "$audit_count" --argjson sealed "$sealed_count" \
    '.applied == false and .report.documents == $documents
      and .report.audit_entries == $entries and .report.sealed_documents == $sealed' \
    "$WORK_DIR/import-inspection.json" >/dev/null
  [ ! -e "$legacy_dir/documents/.migrated" ]
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM documents')" -eq 0 ]
  for _ in 1 2; do
    DATABASE_URL="$imported_url" "$CLI" --json database import --apply \
      --data-dir "$legacy_dir" --mapping "$legacy_dir/mapping.json" >"$WORK_DIR/import-result.json"
    jq -e '.applied == true' "$WORK_DIR/import-result.json" >/dev/null
  done
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM documents')" -eq "$document_count" ]
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM audit_events')" -eq "$((audit_count + 1))" ]
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM migration_receipts')" -eq 1 ]
  test -f "$legacy_dir/documents/.migrated"
  test -f "$legacy_dir/audit.jsonl.migrated"
  psql "$imported_url" -v ON_ERROR_STOP=1 -Atc \
    "SELECT jsonb_build_object('sequence',sequence,'timestamp',timestamp,'actor',actor,
      'action',action,'resource',resource,'chain',encode(chain,'hex'))
      FROM audit_events WHERE sequence < $audit_count ORDER BY sequence" \
    >"$WORK_DIR/imported-audit-prefix.jsonl"
  cmp "$legacy_dir/audit.jsonl" "$WORK_DIR/imported-audit-prefix.jsonl"
  import_report="$(jq -Sc '.report' "$WORK_DIR/import-result.json")"
  [ "$(jq -Sc . "$legacy_dir/documents/.migrated")" = "$import_report" ]

  runtime_url="postgresql://tt_runtime@127.0.0.1:$PG_PORT/imported"
  migration_demo_start "$runtime_url" "$legacy_dir" imported
  migration_demo_export "$case_id" "$WORK_DIR/imported-evidence"
  version_demo "$case_id"
  metadata_demo "$case_id"
  participant_demo "$case_id"
  administration_demo "$case_id"
  stage_demo
  typed_participant_demo "$imported_url"
  hearing_demo "$imported_url"
  migration_demo_stop
  migration_demo_state "$imported_url" >"$WORK_DIR/imported-state.json"
  DATABASE_URL="$imported_url" "$CLI" --json database import --apply \
    --data-dir "$legacy_dir" --mapping "$legacy_dir/mapping.json" >"$WORK_DIR/administration-reconcile.json"
  [ "$(jq -Sc '.report' "$WORK_DIR/administration-reconcile.json")" = "$import_report" ]
  migration_demo_state "$imported_url" >"$WORK_DIR/reconciled-state.json"
  cmp "$WORK_DIR/imported-state.json" "$WORK_DIR/reconciled-state.json"

  pg_dump "$imported_url" --format=custom --file="$WORK_DIR/imported.dump"
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -c 'CREATE DATABASE restored' >/dev/null
  pg_restore --exit-on-error --dbname="$restored_url" "$WORK_DIR/imported.dump"
  migration_demo_state "$restored_url" >"$WORK_DIR/restored-state.json"
  cmp "$WORK_DIR/imported-state.json" "$WORK_DIR/restored-state.json"
  DATABASE_URL="$restored_url" "$CLI" --json database import \
    --data-dir "$legacy_dir" --mapping "$legacy_dir/mapping.json" >"$WORK_DIR/restored-inspection.json"
  [ "$(jq -Sc '.report' "$WORK_DIR/restored-inspection.json")" = "$import_report" ]
  runtime_url="postgresql://tt_runtime@127.0.0.1:$PG_PORT/restored"
  migration_demo_start "$runtime_url" "$legacy_dir" restored
  migration_demo_export "$case_id" "$WORK_DIR/restored-evidence"
  version_demo_restored "$case_id"
  metadata_demo_restored "$case_id"
  participant_demo_restored "$case_id"
  administration_demo_restored
  stage_demo_restored
  typed_participant_demo_restored
  hearing_demo_restored
  printf 'Restored case administration: %s roots, %s revisions, %s initial stage registrations.\n' \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM cases')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_administration_revisions')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_initial_stage_registrations')"
  printf 'Restored inventory: %s document roots, %s content snapshots, %s classification revisions.\n' \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM document_series')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM documents')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM document_metadata_revisions')"
  printf 'Restored participants: %s roots, %s immutable revisions.\n' \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_participants')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_participant_revisions')"
  printf 'Restored typed participants: %s revisions, %s subjects, %s subject revisions, %s credentials.\n' \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_participant_typed_revisions')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_subjects')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_subject_revisions')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM participant_credential_evidence')"
  printf 'Restored hearings: %s roots, %s immutable revisions.\n' \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_hearings')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM case_hearing_revisions')"
  printf 'Migration and restore demo passed: %s documents, %s preserved audit events, identical evidence ZIP.\n' \
    "$document_count" "$audit_count"
}

migration_demo
unset -f migration_demo migration_demo_stop migration_demo_start
unset -f migration_demo_state migration_demo_export
unset -f version_demo version_demo_request version_demo_restored
unset -f metadata_demo metadata_demo_request metadata_demo_restored metadata_demo_evidence
unset -f participant_demo participant_demo_request participant_demo_enroll participant_demo_restored

unset -f administration_demo administration_demo_request administration_demo_body administration_demo_enroll
unset -f administration_demo_closed administration_demo_capture administration_demo_restored
unset -f stage_demo stage_demo_request stage_demo_upload stage_demo_capture stage_demo_restored
unset -f typed_participant_demo typed_participant_demo_restored typed_participant_demo_python
unset -f hearing_demo hearing_demo_restored hearing_demo_python
