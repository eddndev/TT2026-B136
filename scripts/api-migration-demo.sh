#!/usr/bin/env bash
# Rehearse encrypted legacy cutover and backup restoration in api-demo.sh's cluster.

# shellcheck source=scripts/api-version-demo.sh
source "$REPO_ROOT/scripts/api-version-demo.sh"
# shellcheck source=scripts/api-metadata-demo.sh
source "$REPO_ROOT/scripts/api-metadata-demo.sh"

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
      'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a),
      'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY fingerprint) FROM migration_receipts r),
      'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
      'cases',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM cases c),
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
  pg_dump "$DATABASE_ADMIN_URL" --data-only --no-owner --no-privileges \
    -t users -t cases -t case_memberships >"$WORK_DIR/identity-cases.sql"
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -c 'CREATE DATABASE imported' >/dev/null
  DATABASE_URL="$imported_url" "$CLI" database migrate --runtime-role tt_runtime >/dev/null
  psql "$imported_url" -v ON_ERROR_STOP=1 -f "$WORK_DIR/identity-cases.sql" >/dev/null

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
  migration_demo_stop
  migration_demo_state "$imported_url" >"$WORK_DIR/imported-state.json"

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
  printf 'Restored inventory: %s document roots, %s content snapshots, %s classification revisions.\n' \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM document_series')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM documents')" \
    "$(psql "$restored_url" -Atc 'SELECT COUNT(*) FROM document_metadata_revisions')"
  printf 'Migration and restore demo passed: %s documents, %s preserved audit events, identical evidence ZIP.\n' \
    "$document_count" "$audit_count"
}

migration_demo
unset -f migration_demo migration_demo_stop migration_demo_start
unset -f migration_demo_state migration_demo_export
unset -f version_demo version_demo_request version_demo_restored
unset -f metadata_demo metadata_demo_request metadata_demo_restored metadata_demo_evidence
