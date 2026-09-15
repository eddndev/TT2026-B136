#!/usr/bin/env bash
# Audited participant commands and recovery against api-demo.sh's disposable services.

participant_demo_request() {
  local method="$1" expected="$2" route="$3" token="$4" actual
  shift 4
  actual="$(curl -sS -o "$WORK_DIR/participant-response.json" -w '%{http_code}' \
    -X "$method" "$BASE_URL$route" -H "Authorization: Bearer $token" "$@")"
  if [ "$actual" != "$expected" ]; then
    printf 'api-participant-demo.sh: %s %s returned %s, expected %s\n' \
      "$method" "$route" "$actual" "$expected" >&2
    cat "$WORK_DIR/participant-response.json" >&2
    return 1
  fi
}

participant_demo_enroll() {
  local role="$1" enrollment="$WORK_DIR/participant-$1-enrollment.json" challenge code
  curl -fsS -X POST "$BASE_URL/api/v1/users" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" -H 'Content-Type: application/json' \
    --data "$(jq -nc --arg role "$role" \
      '{email:("participant-"+$role+"@example.com"),role:$role,password:"participant demo password"}')" \
    >"$enrollment"
  challenge="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    --data "$(jq -nc --arg role "$role" \
      '{email:("participant-"+$role+"@example.com"),password:"participant demo password"}')" \
    | jq -er '.challenge_token')"
  code="$(jq -er '.recovery_codes[0]' "$enrollment")"
  curl -fsS -X POST "$BASE_URL/api/v1/auth/mfa/recovery" \
    -H 'Content-Type: application/json' \
    --data "$(jq -nc --arg challenge "$challenge" --arg code "$code" \
      '{challenge_token:$challenge,code:$code}')" | jq -er '.access_token'
}

participant_demo() {
  local case_id="$1" route="/api/v1/cases/$1/participants"
  local response="$WORK_DIR/participant-response.json" token user role
  local litigator paralegal client history audit_before statuses left right winner after
  litigator="$(participant_demo_enroll litigator)"
  paralegal="$(participant_demo_enroll paralegal)"
  client="$(participant_demo_enroll client)"
  for role in litigator paralegal client; do
    user="$(jq -er '.user.id' "$WORK_DIR/participant-$role-enrollment.json")"
    participant_demo_request PUT 204 "/api/v1/cases/$case_id/members/$user" "$RECOVERY_TOKEN"
  done
  participant_demo_request POST 201 "$route" "$litigator" -H 'Content-Type: application/json' \
    --data '{"display_name":" Ana ","procedural_role":"Witness","organization":"Office","legal_status":"Recorded"}'
  PARTICIPANT_ID="$(jq -er '.id' "$response")"
  jq -e --arg case_id "$case_id" '.case_id == $case_id and .revision == 1
    and .display_name == "Ana" and .directory_status == "active"
    and .changed_by.email == "participant-litigator@example.com"' "$response" >/dev/null
  participant_demo_request POST 201 "$route" "$RECOVERY_TOKEN" -H 'Content-Type: application/json' \
    --data '{"display_name":"Ana","procedural_role":"Witness"}'
  PARTICIPANT_SECOND_ID="$(jq -er '.id' "$response")"
  [ "$PARTICIPANT_ID" != "$PARTICIPANT_SECOND_ID" ]

  participant_demo_request GET 200 "$route/$PARTICIPANT_ID" "$paralegal"
  for token in "$paralegal" "$client"; do
    participant_demo_request POST 403 "$route" "$token" -H 'Content-Type: application/json' \
      --data '{"display_name":"Denied","procedural_role":"Witness"}'
    participant_demo_request PUT 403 "$route/$PARTICIPANT_ID/directory-status" "$token" \
      -H 'Content-Type: application/json' --data '{"expected_revision":1,"directory_status":"archived"}'
  done
  participant_demo_request GET 403 "$route" "$client"
  participant_demo_request GET 403 "$route/$PARTICIPANT_ID/history" "$client"
  participant_demo_request GET 200 "/api/v1/cases/$case_id" "$client"
  participant_demo_request GET 404 "/api/v1/cases/00000000-0000-4000-8000-000000000000/participants/$PARTICIPANT_ID" "$RECOVERY_TOKEN"

  participant_demo_request PUT 200 "$route/$PARTICIPANT_ID" "$litigator" \
    -H 'Content-Type: application/json' --data '{"expected_revision":1,"display_name":"Ana edited","procedural_role":"Witness","organization":"Office","legal_status":"Recorded","directory_status":"active"}'
  jq -e '.revision == 2 and .display_name == "Ana edited"' "$response" >/dev/null
  audit_before="$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')"
  for side in left right; do
    curl -sS -o "$WORK_DIR/participant-$side.json" -w '%{http_code}' -X PUT \
      "$BASE_URL$route/$PARTICIPANT_ID" -H "Authorization: Bearer $RECOVERY_TOKEN" \
      -H 'Content-Type: application/json' \
      --data "$(jq -nc --arg name "Ana $side" '{expected_revision:2,display_name:$name,
        procedural_role:"Witness",organization:"Office",legal_status:"Recorded",directory_status:"active"}')" \
      >"$WORK_DIR/participant-$side.status" &
    if [ "$side" = left ]; then left=$!; else right=$!; fi
  done
  wait "$left"
  wait "$right"
  statuses="$(cat "$WORK_DIR/participant-left.status" "$WORK_DIR/participant-right.status")"
  [[ "$statuses" = 200409 || "$statuses" = 409200 ]]
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" -eq "$((audit_before+1))" ]
  winner=left
  if [ "$(cat "$WORK_DIR/participant-right.status")" = 200 ]; then winner=right; fi
  jq -e --arg name "Ana $winner" '.revision == 3 and .display_name == $name' \
    "$WORK_DIR/participant-$winner.json" >/dev/null
  participant_demo_request PUT 409 "$route/$PARTICIPANT_ID/directory-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":2,"directory_status":"archived"}'
  jq -e '.error.code == "participant_revision_conflict"' "$response" >/dev/null
  [ "$(psql "$DATABASE_URL" -Atc 'SELECT COUNT(*) FROM audit_events')" -eq "$((audit_before+1))" ]
  participant_demo_request PUT 200 "$route/$PARTICIPANT_ID/directory-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":3,"directory_status":"archived"}'
  jq -e --arg name "Ana $winner" '.revision == 4 and .display_name == $name
    and .organization == "Office" and .legal_status == "Recorded"
    and .directory_status == "archived"' "$response" >/dev/null
  participant_demo_request GET 200 "$route?status=archived&name=Ana&procedural_role=Witness" "$paralegal"
  jq -e --arg id "$PARTICIPANT_ID" '(.participants | map(.id)) == [$id]' "$response" >/dev/null
  participant_demo_request GET 200 "$route?name=ana" "$paralegal"
  jq -e '.participants == []' "$response" >/dev/null
  participant_demo_request GET 200 "$route" "$paralegal"
  jq -e --arg id "$PARTICIPANT_SECOND_ID" '(.participants | map(.id)) == [$id]' "$response" >/dev/null

  participant_demo_request PUT 200 "$route/$PARTICIPANT_ID/directory-status" "$RECOVERY_TOKEN" \
    -H 'Content-Type: application/json' --data '{"expected_revision":4,"directory_status":"active"}'
  participant_demo_request GET 200 "$route?limit=1" "$paralegal"
  after="$(jq -er '.next_after_id' "$response")"
  participant_demo_request GET 200 "$route?limit=1&after_id=$after" "$paralegal"
  jq -e --arg after "$after" '.has_more == false and .next_after_id == null
    and (.participants | length) == 1 and .participants[0].id != $after' "$response" >/dev/null

  user="$(jq -er '.user.id' "$WORK_DIR/participant-paralegal-enrollment.json")"
  participant_demo_request DELETE 204 "/api/v1/cases/$case_id/members/$user" "$RECOVERY_TOKEN"
  participant_demo_request GET 404 "$route" "$paralegal"
  participant_demo_request GET 404 "$route/$PARTICIPANT_ID" "$paralegal"
  participant_demo_request GET 404 "$route/$PARTICIPANT_ID/history" "$paralegal"
  participant_demo_request GET 200 "$route/$PARTICIPANT_ID/history?limit=2&before_revision=5" "$RECOVERY_TOKEN"
  jq -e '(.revisions | map(.revision)) == [4,3] and .next_before_revision == 3 and .has_more' "$response" >/dev/null
  participant_demo_request GET 200 "$route/$PARTICIPANT_ID/history" "$RECOVERY_TOKEN"
  jq -e '(.revisions | map(.revision)) == [5,4,3,2,1]
    and .revisions[-1].changed_by.email == "participant-litigator@example.com"
    and .revisions[0].changed_by.email == "owner@example.com"' "$response" >/dev/null
  jq -Sc . "$response" >"$WORK_DIR/participant-history.json"
  participant_demo_request GET 200 "$route?status=all" "$RECOVERY_TOKEN"
  jq -Sc . "$response" >"$WORK_DIR/participant-list.json"
  printf 'Participant API demo passed: role isolation, homonyms, revision race, status preservation and revocation.\n'
}

participant_demo_restored() {
  local route="/api/v1/cases/$1/participants" response="$WORK_DIR/participant-response.json"
  participant_demo_request GET 200 "$route/$PARTICIPANT_ID/history" "$RECOVERY_TOKEN"
  jq -Sc . "$response" >"$WORK_DIR/restored-participant-history.json"
  cmp "$WORK_DIR/participant-history.json" "$WORK_DIR/restored-participant-history.json"
  participant_demo_request GET 200 "$route?status=all" "$RECOVERY_TOKEN"
  jq -Sc . "$response" >"$WORK_DIR/restored-participant-list.json"
  cmp "$WORK_DIR/participant-list.json" "$WORK_DIR/restored-participant-list.json"
  printf 'Participant restore demo passed: all revisions, values, actors and timestamps preserved.\n'
}
