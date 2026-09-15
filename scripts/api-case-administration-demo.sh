#!/usr/bin/env bash
# Penal registration and case administration in api-demo.sh's disposable services.

administration_demo_request() {
  local method="$1" expected="$2" route="$3" token="$4" actual
  shift 4
  actual="$(curl -sS -o "$WORK_DIR/administration-response.json" -w '%{http_code}' \
    -X "$method" "$BASE_URL$route" -H "Authorization: Bearer $token" "$@")"
  if [ "$actual" != "$expected" ]; then
    printf 'api-case-administration-demo.sh: %s %s returned %s, expected %s\n' \
      "$method" "$route" "$actual" "$expected" >&2
    cat "$WORK_DIR/administration-response.json" >&2
    return 1
  fi
}

administration_demo_body() {
  jq -nc --arg title "$1" --arg nuc "$2" --arg judicial "$3" --argjson expected "${4:-null}" \
    '{title:$title,reference:"ADMIN-REF",profile:{nuc:$nuc,nuc_authority:"Fiscalia registrada",
      judicial_case_number:$judicial,judicial_authority:"Organo registrado",
      offenses:["Offense A","Offense B"],general_information:" First line\r\nSecond line ",
      complementary_identifiers:null}} +
      (if $expected == null then {} else {expected_revision:$expected} end)'
}

administration_demo_enroll() {
  local role="$1" enrollment="$WORK_DIR/administration-$1-enrollment.json" challenge code
  curl -fsS -X POST "$BASE_URL/api/v1/users" -H "Authorization: Bearer $RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(jq -nc --arg role "$role" \
      '{email:("administration-"+$role+"@example.com"),role:$role,password:"administration demo password"}')" \
    >"$enrollment"
  challenge="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/login" -H 'Content-Type: application/json' \
    --data "$(jq -nc --arg role "$role" \
      '{email:("administration-"+$role+"@example.com"),password:"administration demo password"}')" \
    | jq -er '.challenge_token')"
  code="$(jq -er '.recovery_codes[0]' "$enrollment")"
  curl -fsS -X POST "$BASE_URL/api/v1/auth/mfa/recovery" -H 'Content-Type: application/json' \
    --data "$(jq -nc --arg token "$challenge" --arg code "$code" '{challenge_token:$token,code:$code}')" \
    | jq -er '.access_token'
}

administration_demo_closed() {
  local id="$1" route="/api/v1/cases/$1" response="$WORK_DIR/administration-response.json"
  local before participant_revision endpoint
  administration_demo_request GET 200 "$route/participants/$PARTICIPANT_ID" "$RECOVERY_TOKEN"
  participant_revision="$(jq -er '.revision' "$response")"
  jq -Sc . "$response" >"$WORK_DIR/administration-participant-before.json"
  administration_demo_request PUT 200 "$route/administrative-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":0,"administrative_status":"closed"}'
  jq -e '.administration.revision == 1 and .administration.profile == null
    and .administration.administrative_status == "closed" and .initial_stage == null' "$response" >/dev/null
  before="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  administration_demo_request POST 409 "$route/documents" "$RECOVERY_TOKEN" \
    -H 'X-Document-Name: denied.txt' --data-binary "@$WORK_DIR/document.txt"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request POST 409 "$route/documents/with-metadata" "$RECOVERY_TOKEN" \
    -H 'X-Document-Name: denied.txt' -F 'metadata={"tags":[]};type=application/json' \
    -F "file=@$WORK_DIR/document.txt;filename=denied.txt;type=application/octet-stream"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request POST 409 "$route/documents/$DOCUMENT_ID/versions?expected_version=2" \
    "$RECOVERY_TOKEN" -H 'X-Document-Name: denied.txt' --data-binary "@$WORK_DIR/document.txt"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request PUT 409 "$route/documents/$DOCUMENT_ID/metadata" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_metadata_revision":2,"tags":[]}'
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request POST 409 "$route/documents/$METADATA_DOCUMENT_ID/versions/1/seal" "$RECOVERY_TOKEN"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request POST 409 "$route/participants" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"display_name":"Denied","procedural_role":"Witness"}'
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request PUT 409 "$route/participants/$PARTICIPANT_ID" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(jq -nc --argjson revision "$participant_revision" \
      '{expected_revision:$revision,display_name:"Denied",procedural_role:"Witness",directory_status:"active"}')"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request PUT 409 "$route/participants/$PARTICIPANT_ID/directory-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(jq -nc --argjson revision "$participant_revision" \
      '{expected_revision:$revision,directory_status:"archived"}')"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  administration_demo_request PUT 409 "$route/administration" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(administration_demo_body Denied NUC-LEGACY CJ-LEGACY 1)"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" = "$before" ]
  for endpoint in documents "documents/$DOCUMENT_ID/versions" participants administration \
    administration/history "documents/$DOCUMENT_ID/metadata/history" "participants/$PARTICIPANT_ID/history"; do
    administration_demo_request GET 200 "$route/$endpoint" "$RECOVERY_TOKEN"
  done
  administration_demo_request POST 200 "$route/documents/$DOCUMENT_ID/versions/1/verify" "$RECOVERY_TOKEN"
  jq -e '.verdict == "valid"' "$response" >/dev/null
  metadata_demo_evidence "$id" administration-closed
  administration_demo_request PUT 200 "$route/administrative-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":1,"administrative_status":"active"}'
  jq -e '.administration.revision == 2 and .administration.profile == null
    and .administration.administrative_status == "active" and .initial_stage == null' "$response" >/dev/null
  administration_demo_request GET 200 "$route/participants/$PARTICIPANT_ID" "$RECOVERY_TOKEN"
  jq -Sc . "$response" >"$WORK_DIR/administration-participant-after.json"
  cmp "$WORK_DIR/administration-participant-before.json" "$WORK_DIR/administration-participant-after.json"
}

administration_demo_capture() {
  local label="$1" id suffix response="$WORK_DIR/administration-response.json"
  for id in $ADMINISTRATION_CASE_IDS; do
    for suffix in administration administration/history; do
      administration_demo_request GET 200 "/api/v1/cases/$id/$suffix" "$RECOVERY_TOKEN"
      jq -Sc . "$response" >"$WORK_DIR/$label-$id-${suffix//\//-}.json"
      if [ "$label" = restored-administration ]; then
        cmp "$WORK_DIR/administration-$id-${suffix//\//-}.json" \
          "$WORK_DIR/$label-$id-${suffix//\//-}.json"
      fi
    done
  done
}

administration_demo() {
  local legacy="$1" response="$WORK_DIR/administration-response.json"
  local litigator paralegal client user role token basic penal reused before title stage
  local left right winner statuses hidden first route input side roots
  litigator="$(administration_demo_enroll litigator)"
  paralegal="$(administration_demo_enroll paralegal)"
  client="$(administration_demo_enroll client)"
  administration_demo_request GET 200 "/api/v1/cases/$legacy/administration" "$RECOVERY_TOKEN"
  jq -e '.administration.revision == 0 and .administration.profile == null
    and .administration.changed_at == null and .administration.changed_by == null
    and .administration.values_digest == null and .initial_stage == null' "$response" >/dev/null
  administration_demo_request GET 200 "/api/v1/cases/$legacy/administration/history" "$RECOVERY_TOKEN"
  jq -e '.revisions == [] and .has_more == false' "$response" >/dev/null

  before="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  roots="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM cases')"
  for input in "$(administration_demo_body Invalid ' ' CJ-INVALID)" \
    "$(administration_demo_body Invalid NUC-INVALID CJ-INVALID | jq '.profile.offenses = ["same"," same "]')" \
    "$(administration_demo_body Invalid NUC-INVALID CJ-INVALID | jq '.profile.general_information = "isolated\rcarriage"')"; do
    administration_demo_request POST 422 /api/v1/penal-cases "$RECOVERY_TOKEN" \
      -H 'Content-Type: application/json' --data "$input"
  done
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM cases')" = "$roots" ]
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" = "$before" ]

  before="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  administration_demo_request POST 201 /api/v1/cases "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"title":"Basic pending","reference":"BASIC"}'
  basic="$(jq -er '.id' "$response")"
  jq -e 'keys == ["created_by","id","reference","title"]' "$response" >/dev/null
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" -eq "$((before+3))" ]
  administration_demo_request GET 200 "/api/v1/cases/$basic/administration" "$RECOVERY_TOKEN"
  jq -e '.administration.revision == 1 and .administration.profile == null
    and .administration.changed_by != null and .initial_stage == null' "$response" >/dev/null
  input="$(administration_demo_body ' Penal complete ' NUC-ADMIN CJ-ADMIN)"
  before="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  administration_demo_request POST 201 /api/v1/penal-cases "$litigator" \
    -H 'Content-Type: application/json' --data "$input"
  penal="$(jq -er '.id' "$response")"
  jq -e '.administration.revision == 1 and .administration.title == "Penal complete"
    and .administration.administrative_status == "active"
    and .administration.profile.general_information == "First line\nSecond line"
    and .administration.profile.offenses == ["Offense A","Offense B"]
    and .initial_stage.stage == "investigation" and .initial_stage.stage_revision == 1
    and .initial_stage.administration_revision == 1
    and .initial_stage.recorded_by == .administration.changed_by
    and .initial_stage.recorded_at == .administration.changed_at
    and .initial_stage.administration_digest == .administration.values_digest
    and .created_by == .administration.changed_by.id' "$response" >/dev/null
  stage="$(jq -Sc '.initial_stage' "$response")"
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" -eq "$((before+4))" ]
  for role in paralegal client; do
    user="$(jq -er '.user.id' "$WORK_DIR/administration-$role-enrollment.json")"
    administration_demo_request PUT 204 "/api/v1/cases/$penal/members/$user" "$RECOVERY_TOKEN"
  done
  route="/api/v1/cases/$penal"
  administration_demo_request GET 200 "$route/administration" "$paralegal"
  administration_demo_request GET 200 "$route/administration/history" "$paralegal"
  administration_demo_request GET 200 "$route" "$client"
  jq -e 'keys == ["created_by","id","reference","title"]' "$response" >/dev/null
  for endpoint in /api/v1/case-administrations "$route/administration" "$route/administration/history"; do
    administration_demo_request GET 403 "$endpoint" "$client"
  done
  for token in "$paralegal" "$client"; do
    administration_demo_request POST 403 /api/v1/penal-cases "$token" -H 'Content-Type: application/json' --data "$input"
    administration_demo_request PUT 403 "$route/administration" "$token" -H 'Content-Type: application/json' \
      --data "$(administration_demo_body Denied NUC-ADMIN CJ-ADMIN 1)"
    administration_demo_request PUT 403 "$route/administrative-status" "$token" \
      -H 'Content-Type: application/json' --data '{"expected_revision":1,"administrative_status":"closed"}'
  done
  administration_demo_request GET 404 "/api/v1/cases/$legacy/administration" "$litigator"
  hidden="$(jq -Sc . "$response")"
  administration_demo_request GET 404 /api/v1/cases/00000000-0000-4000-8000-000000000000/administration "$litigator"
  [ "$(jq -Sc . "$response")" = "$hidden" ]
  for input in "$(administration_demo_body Duplicate NUC-ADMIN CJ-OTHER)" \
    "$(administration_demo_body Duplicate NUC-OTHER CJ-ADMIN)"; do
    administration_demo_request POST 409 /api/v1/penal-cases "$RECOVERY_TOKEN" \
      -H 'Content-Type: application/json' --data "$input"
    jq -e '.error.code == "case_identifier_conflict"' "$response" >/dev/null
  done

  before="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  for side in left right; do
    curl -sS -o "$WORK_DIR/administration-$side.json" -w '%{http_code}' -X PUT \
      "$BASE_URL$route/administration" -H "Authorization: Bearer $RECOVERY_TOKEN" \
      -H 'Content-Type: application/json' \
      --data "$(administration_demo_body "Penal $side" NUC-ADMIN CJ-ADMIN 1)" \
      >"$WORK_DIR/administration-$side.status" &
    if [ "$side" = left ]; then left=$!; else right=$!; fi
  done
  wait "$left"
  wait "$right"
  statuses="$(cat "$WORK_DIR/administration-left.status" "$WORK_DIR/administration-right.status")"
  [[ "$statuses" = 200409 || "$statuses" = 409200 ]]
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" -eq "$((before+1))" ]
  winner=left
  if [ "$(cat "$WORK_DIR/administration-right.status")" = 200 ]; then winner=right; fi
  title="Penal $winner"
  jq -e --arg title "$title" '.administration.revision == 2 and .administration.title == $title' \
    "$WORK_DIR/administration-$winner.json" >/dev/null
  administration_demo_request PUT 409 "$route/administrative-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":1,"administrative_status":"closed"}'
  jq -e '.error.code == "case_revision_conflict"' "$response" >/dev/null
  administration_demo_request GET 200 "$route" "$client"
  jq -e --arg title "$title" '.title == $title and keys == ["created_by","id","reference","title"]' "$response" >/dev/null
  administration_demo_request PUT 200 "$route/administrative-status" "$litigator" \
    -H 'Content-Type: application/json' --data '{"expected_revision":2,"administrative_status":"closed"}'
  jq -e --arg title "$title" '.administration.revision == 3 and .administration.title == $title
    and .administration.administrative_status == "closed"' "$response" >/dev/null
  [ "$(jq -Sc '.initial_stage' "$response")" = "$stage" ]
  administration_demo_request POST 409 /api/v1/penal-cases "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(administration_demo_body Duplicate NUC-ADMIN CJ-DUPLICATE)"
  jq -e '.error.code == "case_identifier_conflict"' "$response" >/dev/null
  administration_demo_request POST 409 "$route/documents" "$RECOVERY_TOKEN" \
    -H 'X-Document-Name: denied.txt' --data-binary "@$WORK_DIR/document.txt"
  jq -e '.error.code == "case_closed"' "$response" >/dev/null
  user="$(jq -er '.user.id' "$WORK_DIR/administration-client-enrollment.json")"
  administration_demo_request DELETE 204 "$route/members/$user" "$RECOVERY_TOKEN"
  administration_demo_request PUT 204 "$route/members/$user" "$RECOVERY_TOKEN"
  administration_demo_request GET 200 "$route" "$client"
  administration_demo_request GET 200 '/api/v1/case-administrations?status=closed&profile=complete&nuc=NUC-ADMIN&judicial_case_number=CJ-ADMIN' "$paralegal"
  jq -e --arg id "$penal" '(.cases | map(.id)) == [$id] and .cases[0].initial_stage == "investigation"' "$response" >/dev/null
  administration_demo_request GET 200 '/api/v1/case-administrations?status=all&nuc=nuc-admin' "$paralegal"
  jq -e '.cases == []' "$response" >/dev/null
  administration_demo_request GET 200 '/api/v1/case-administrations?status=all&limit=1' "$RECOVERY_TOKEN"
  first="$(jq -er '.next_after_id' "$response")"
  administration_demo_request GET 200 "/api/v1/case-administrations?status=all&limit=1&after_id=$first" "$RECOVERY_TOKEN"
  jq -e --arg first "$first" '(.cases | length) == 1 and .cases[0].id > $first' "$response" >/dev/null

  administration_demo_closed "$legacy"
  administration_demo_request PUT 200 "/api/v1/cases/$legacy/administration" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(administration_demo_body 'Legacy completed' NUC-LEGACY CJ-LEGACY 2)"
  jq -e '.administration.revision == 3 and .administration.profile.nuc == "NUC-LEGACY"
    and .initial_stage == null' "$response" >/dev/null
  administration_demo_request PUT 200 "/api/v1/cases/$basic/administration" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(administration_demo_body 'Basic completed' NUC-BASIC CJ-BASIC 1)"
  jq -e '.administration.revision == 2 and .administration.profile != null and .initial_stage == null' "$response" >/dev/null
  administration_demo_request PUT 409 "/api/v1/cases/$basic/administration" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":2,"title":"Basic","reference":"BASIC","profile":null}'
  jq -e '.error.code == "case_profile_required"' "$response" >/dev/null
  administration_demo_request PUT 200 "$route/administrative-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":3,"administrative_status":"active"}'
  administration_demo_request PUT 200 "$route/administration" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(administration_demo_body "$title" NUC-CORRECTED CJ-CORRECTED 4)"
  jq -e '.administration.revision == 5 and .administration.profile.nuc == "NUC-CORRECTED"' "$response" >/dev/null
  [ "$(jq -Sc '.initial_stage' "$response")" = "$stage" ]
  administration_demo_request POST 201 /api/v1/penal-cases "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data "$(administration_demo_body 'Released identifiers' NUC-ADMIN CJ-ADMIN)"
  reused="$(jq -er '.id' "$response")"
  administration_demo_request GET 200 "$route/administration/history?limit=2&before_revision=5" "$RECOVERY_TOKEN"
  jq -e '(.revisions | map(.revision)) == [4,3] and .has_more and .next_before_revision == 3' "$response" >/dev/null
  user="$(jq -er '.user.id' "$WORK_DIR/administration-litigator-enrollment.json")"
  administration_demo_request DELETE 204 "$route/members/$user" "$RECOVERY_TOKEN"
  administration_demo_request GET 404 "$route/administration" "$litigator"
  administration_demo_request GET 404 "$route/administration/history" "$litigator"
  administration_demo_request GET 200 '/api/v1/case-administrations?status=all' "$litigator"
  jq -e '.cases == []' "$response" >/dev/null
  ADMINISTRATION_CASE_IDS="$legacy $basic $penal $reused"
  administration_demo_capture administration
  printf 'Case administration API demo passed: penal R1, basic R1, legacy R0, four roles, CAS, close gates, identifier reuse, immutable initial stage.\n'
}

administration_demo_restored() {
  administration_demo_capture restored-administration
  printf 'Case administration restore demo passed: original creation facts, current details, all administrative revisions and initial stage snapshots preserved.\n'
}
