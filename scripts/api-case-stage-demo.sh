#!/usr/bin/env bash
# Exact stage supports and restore checks in api-demo.sh's disposable services.

stage_demo_request() {
  local method="$1" expected="$2" route="$3" token="$4" actual
  shift 4
  actual="$(curl -sS -o "$WORK_DIR/stage-response.json" -w '%{http_code}' \
    -X "$method" "$BASE_URL$route" -H "Authorization: Bearer $token" "$@")"
  if [ "$actual" != "$expected" ]; then
    printf 'api-case-stage-demo.sh: %s %s returned %s, expected %s\n' \
      "$method" "$route" "$actual" "$expected" >&2
    cat "$WORK_DIR/stage-response.json" >&2
    return 1
  fi
}

stage_demo_upload() {
  local case_id="$1" path="$2" name="$3"
  stage_demo_request POST 201 "/api/v1/cases/$case_id/documents" "$RECOVERY_TOKEN" \
    -H "X-Document-Name: $name" --data-binary "@$path"
  jq -c '{document_id:.id,version:.version,digest:.digest}' "$WORK_DIR/stage-response.json"
}

stage_demo_capture() {
  local label="$1" id suffix
  for id in "$STAGE_DEMO_CASE_ID" "$STAGE_DEMO_ADOPTION_ID"; do
    for suffix in stage stage/history administration; do
      stage_demo_request GET 200 "/api/v1/cases/$id/$suffix" "$RECOVERY_TOKEN"
      jq -Sc . "$WORK_DIR/stage-response.json" >"$WORK_DIR/$label-$id-${suffix//\//-}.json"
      if [ "$label" = restored-stage ]; then
        cmp "$WORK_DIR/stage-$id-${suffix//\//-}.json" "$WORK_DIR/$label-$id-${suffix//\//-}.json"
      fi
    done
  done
  curl -fsS "$BASE_URL/api/v1/cases/$STAGE_DEMO_CASE_ID/documents/$STAGE_DEMO_SUPPORT_ID/versions/1/evidence" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" -o "$WORK_DIR/$label-evidence.zip"
  cmp "$WORK_DIR/stage-original-evidence.zip" "$WORK_DIR/$label-evidence.zip"
}

stage_demo() {
  local response="$WORK_DIR/stage-response.json" route support receipt invalid adoption
  local before request left right statuses side paralegal_id hidden initial
  local pdf="$REPO_ROOT/crates/infrastructure/tests/fixtures/stage-support.pdf"
  local docx="$REPO_ROOT/crates/infrastructure/src/document_formats/docx/tests/fixtures/producer.docx"
  stage_demo_request POST 201 /api/v1/penal-cases "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' \
    --data "$(administration_demo_body 'Process stage case' NUC-STAGES CJ-STAGES)"
  STAGE_DEMO_CASE_ID="$(jq -er '.id' "$response")"
  initial="$(jq -Sc '.initial_stage' "$response")"
  route="/api/v1/cases/$STAGE_DEMO_CASE_ID"
  paralegal_id="$(jq -er '.user.id' <<<"$PARALEGAL")"
  stage_demo_request PUT 204 "$route/members/$paralegal_id" "$RECOVERY_TOKEN"
  stage_demo_request GET 200 "$route/stage" "$PARALEGAL_TOKEN"
  jq -e '.current.kind == "initial" and .current.stage_revision == 1' "$response" >/dev/null

  support="$(stage_demo_upload "$STAGE_DEMO_CASE_ID" "$pdf" accusation.pdf)"
  STAGE_DEMO_SUPPORT_ID="$(jq -er '.document_id' <<<"$support")"
  invalid="$(stage_demo_upload "$STAGE_DEMO_CASE_ID" "$WORK_DIR/document.txt" invalid.pdf)"
  request="$(jq -nc --argjson support "$support" '{expected_revision:1,target:"intermediate",
    accusation_declared_at:{precision:"instant",at:"2026-09-01T10:30:00-06:00"},accusation:$support,note:"Declared accusation"}')"
  before="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  stage_demo_request POST 403 "$route/stage/transitions" "$PARALEGAL_TOKEN" \
    -H 'Content-Type: application/json' --data "$request"
  stage_demo_request POST 422 "$route/stage/transitions" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(jq --argjson bad "$invalid" '.accusation=$bad' <<<"$request")"
  jq -e '.error.code == "stage_support_format_rejected"' "$response" >/dev/null
  stage_demo_request POST 422 "$route/stage/transitions" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(jq '.accusation_declared_at.at="9999-01-01T00:00:00Z"' <<<"$request")"
  jq -e '.error.code == "stage_act_in_future"' "$response" >/dev/null
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" = "$before" ]

  for side in left right; do
    curl -sS -o "$WORK_DIR/stage-$side.json" -w '%{http_code}' -X POST \
      "$BASE_URL$route/stage/transitions" -H "Authorization: Bearer $RECOVERY_TOKEN" \
      -H 'Content-Type: application/json' --data "$request" >"$WORK_DIR/stage-$side.status" &
    if [ "$side" = left ]; then left=$!; else right=$!; fi
  done
  wait "$left"
  wait "$right"
  statuses="$(cat "$WORK_DIR/stage-left.status" "$WORK_DIR/stage-right.status")"
  [[ "$statuses" = 201409 || "$statuses" = 409201 ]]
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" -eq "$((before+1))" ]
  stage_demo_request GET 200 "$route/stage" "$RECOVERY_TOKEN"
  jq -e --argjson support "$support" '.current.stage_revision == 2 and .current.stage == "intermediate"
    and .current.values.accusation == $support and .current.supports[0].format == "pdf"
    and .current.values.accusation_declared_at.at == "2026-09-01T10:30:00-06:00"' "$response" >/dev/null

  stage_demo_request POST 200 "$route/documents/$STAGE_DEMO_SUPPORT_ID/versions/1/seal" "$RECOVERY_TOKEN"
  curl -fsS "$BASE_URL$route/documents/$STAGE_DEMO_SUPPORT_ID/versions/1/evidence" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" -o "$WORK_DIR/stage-original-evidence.zip"
  stage_demo_request POST 201 "$route/documents/$STAGE_DEMO_SUPPORT_ID/versions?expected_version=1" "$RECOVERY_TOKEN" \
    -H 'X-Document-Name: later.txt' --data-binary "@$WORK_DIR/document.txt"
  receipt="$(stage_demo_upload "$STAGE_DEMO_CASE_ID" "$docx" receipt.docx)"
  request="$(jq -nc --argjson support "$support" --argjson receipt "$receipt" '{expected_revision:2,target:"trial",
    opening_order_issued_at:{precision:"date",date:"2026-09-01",offset:"-06:00"},opening_order:$support,
    received_at:{precision:"instant",at:"2026-09-01T15:10:00-06:00"},receiving_court:"Tribunal declarado",
    receipt_reference:"REC-001",receipt_support:$receipt,note:null}')"
  stage_demo_request POST 201 "$route/stage/transitions" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$request"
  jq -e '.current.stage_revision == 3 and .current.stage == "trial"
    and (.current.supports | map(.format)) == ["pdf","docx"]
    and (.current.supports | map(.version)) == [1,1]
    and .current.administration_revision == 1' "$response" >/dev/null
  stage_demo_request GET 200 "$route/stage/history?limit=2" "$RECOVERY_TOKEN"
  jq -e '(.entries | map(.stage_revision)) == [3,2] and .has_more and .next_before_revision == 2' "$response" >/dev/null
  stage_demo_request GET 200 "$route/stage/history?limit=2&before_revision=2" "$RECOVERY_TOKEN"
  jq -e '(.entries | map(.stage_revision)) == [1] and .entries[0].kind == "initial"
    and .has_more == false and .next_before_revision == null' "$response" >/dev/null
  stage_demo_request GET 200 "$route/administration" "$RECOVERY_TOKEN"
  [ "$(jq -Sc '.initial_stage' "$response")" = "$initial" ]
  stage_demo_request PUT 200 "$route/administrative-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":1,"administrative_status":"closed"}'
  stage_demo_request GET 200 "$route/stage" "$PARALEGAL_TOKEN"
  jq -e '.current.stage == "trial" and .current.administration_revision == 1' "$response" >/dev/null
  stage_demo_request DELETE 204 "$route/members/$paralegal_id" "$RECOVERY_TOKEN"
  stage_demo_request GET 404 "$route/stage" "$PARALEGAL_TOKEN"
  hidden="$(jq -Sc . "$response")"
  stage_demo_request GET 404 /api/v1/cases/00000000-0000-4000-8000-000000000000/stage "$PARALEGAL_TOKEN"
  [ "$(jq -Sc . "$response")" = "$hidden" ]

  stage_demo_request POST 201 /api/v1/cases "$RECOVERY_TOKEN" -H 'Content-Type: application/json' \
    --data '{"title":"Adopted procedural case","reference":"STAGE-ADOPTION"}'
  STAGE_DEMO_ADOPTION_ID="$(jq -er '.id' "$response")"
  route="/api/v1/cases/$STAGE_DEMO_ADOPTION_ID"
  stage_demo_request PUT 200 "$route/administration" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' \
    --data "$(administration_demo_body 'Adopted procedural case' NUC-STAGE-ADOPTION CJ-STAGE-ADOPTION 1)"
  stage_demo_request GET 200 "$route/stage" "$RECOVERY_TOKEN"
  jq -e '.current == null' "$response" >/dev/null
  adoption="$(jq -nc --argjson support "$support" '{expected_revision:0,stage:"intermediate",
    known_at:{precision:"date",date:"2026-09-01",offset:"-06:00"},reason:"Incorporated at known stage",support:$support}')"
  stage_demo_request POST 404 "$route/stage/adoption" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$adoption"
  support="$(stage_demo_upload "$STAGE_DEMO_ADOPTION_ID" "$pdf" adoption.pdf)"
  adoption="$(jq --argjson support "$support" '.support=$support' <<<"$adoption")"
  stage_demo_request POST 201 "$route/stage/adoption" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$adoption"
  jq -e '.current.stage_revision == 1 and .current.from_stage == null
    and .current.values.kind == "adoption" and .current.administration_revision == 2' "$response" >/dev/null
  stage_demo_request POST 409 "$route/stage/adoption" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$adoption"
  stage_demo_capture stage
  printf 'Case stage API demo passed: actual PDF/DOCX, exact V1 after V2, adoption, CAS, permissions and preserved evidence.\n'
}

stage_demo_restored() {
  stage_demo_capture restored-stage
  printf 'Case stage restore demo passed: complete stage snapshots, dates, actors, initial origin and original evidence ZIP preserved.\n'
}
