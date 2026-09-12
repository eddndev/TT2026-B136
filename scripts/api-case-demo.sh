#!/usr/bin/env bash
# Sourced by api-demo.sh against its disposable PostgreSQL and Redis services.

case_demo_request() {
  local method="$1" expected="$2" route="$3" token="$4" actual
  shift 4
  actual="$(curl -sS -o "$case_response" -w '%{http_code}' \
    -X "$method" "$BASE_URL$route" -H "Authorization: Bearer $token" "$@")"
  if [ "$actual" != "$expected" ]; then
    printf 'api-case-demo.sh: %s %s returned %s, expected %s\n' \
      "$method" "$route" "$actual" "$expected" >&2
    return 1
  fi
}

case_demo_enroll() {
  local email="$1" role="$2"
  curl -fsS -X POST "$BASE_URL/api/v1/users" \
    -H "Authorization: Bearer $OWNER_TOKEN" -H 'Content-Type: application/json' \
    --data "$(jq -n --arg email "$email" --arg role "$role" \
      '{email: $email, role: $role, password: "case demo safe password"}')"
}

case_demo_login() {
  local enrollment="$1" email challenge code
  email="$(jq -er '.user.email' <<<"$enrollment")"
  code="$(jq -er '.recovery_codes[0]' <<<"$enrollment")"
  challenge="$(curl -fsS -X POST "$BASE_URL/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    --data "$(jq -n --arg email "$email" \
      '{email: $email, password: "case demo safe password"}')" \
    | jq -er '.challenge_token')"
  curl -fsS -X POST "$BASE_URL/api/v1/auth/mfa/recovery" \
    -H 'Content-Type: application/json' \
    --data "$(jq -n --arg challenge "$challenge" --arg code "$code" \
      '{challenge_token: $challenge, code: $code}')" | jq -er '.access_token'
}

case_demo() {
  local case_response="$WORK_DIR/case-response.json"
  local litigator client litigator_token client_token paralegal_id client_id
  local owner_case litigator_case first_page_id second_page_id hidden_error
  local missing_case="00000000-0000-4000-8000-000000000000"

  litigator="$(case_demo_enroll litigator-cases@example.com litigator)"
  client="$(case_demo_enroll client-cases@example.com client)"
  litigator_token="$(case_demo_login "$litigator")"
  client_token="$(case_demo_login "$client")"
  paralegal_id="$(jq -er '.user.id' <<<"$PARALEGAL")"
  client_id="$(jq -er '.user.id' <<<"$client")"

  case_demo_request POST 201 /api/v1/cases "$OWNER_TOKEN" \
    -H 'Content-Type: application/json' \
    --data "$(jq -n '{title: "  Owner case  ", reference: "  NUC-DEMO-1  "}')"
  owner_case="$(jq -er '.id' "$case_response")"
  jq -e '.title == "Owner case" and .reference == "NUC-DEMO-1"' \
    "$case_response" >/dev/null

  case_demo_request POST 201 /api/v1/cases "$litigator_token" \
    -H 'Content-Type: application/json' \
    --data "$(jq -n '{title: "Litigator case", reference: "NUC-DEMO-2"}')"
  litigator_case="$(jq -er '.id' "$case_response")"
  case_demo_request GET 200 "/api/v1/cases/$litigator_case" "$litigator_token"
  jq -e --arg id "$litigator_case" '.id == $id' "$case_response" >/dev/null

  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=0' "$litigator_token"
  jq -e --arg id "$litigator_case" \
    'length == 1 and .[0].id == $id' "$case_response" >/dev/null
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=1' "$litigator_token"
  jq -e 'length == 0' "$case_response" >/dev/null
  case_demo_request GET 404 "/api/v1/cases/$owner_case" "$litigator_token"
  jq -e '.error.code == "case_not_found"' "$case_response" >/dev/null
  hidden_error="$(jq -Sc . "$case_response")"
  case_demo_request GET 404 "/api/v1/cases/$missing_case" "$litigator_token"
  [ "$(jq -Sc . "$case_response")" = "$hidden_error" ]

  case_demo_request GET 200 /api/v1/cases "$OWNER_TOKEN"
  jq -e --arg first "$owner_case" --arg second "$litigator_case" \
    'length == 2 and ([.[].id] | sort) == ([$first, $second] | sort)' \
    "$case_response" >/dev/null
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=0' "$OWNER_TOKEN"
  jq -e 'length == 1' "$case_response" >/dev/null
  first_page_id="$(jq -er '.[0].id' "$case_response")"
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=1' "$OWNER_TOKEN"
  jq -e 'length == 1' "$case_response" >/dev/null
  second_page_id="$(jq -er '.[0].id' "$case_response")"
  [ "$first_page_id" != "$second_page_id" ]
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=2' "$OWNER_TOKEN"
  jq -e 'length == 0' "$case_response" >/dev/null

  case_demo_request POST 403 /api/v1/cases "$PARALEGAL_TOKEN" \
    -H 'Content-Type: application/json' \
    --data "$(jq -n '{title: "Denied case", reference: "NUC-DENIED"}')"
  case_demo_request GET 200 /api/v1/cases "$PARALEGAL_TOKEN"
  jq -e 'length == 0' "$case_response" >/dev/null
  case_demo_request PUT 204 "/api/v1/cases/$owner_case/members/$paralegal_id" \
    "$OWNER_TOKEN"
  case_demo_request PUT 204 "/api/v1/cases/$owner_case/members/$paralegal_id" \
    "$OWNER_TOKEN"
  case_demo_request GET 200 "/api/v1/cases/$owner_case" "$PARALEGAL_TOKEN"
  case_demo_request GET 200 /api/v1/cases "$PARALEGAL_TOKEN"
  jq -e --arg id "$owner_case" \
    'length == 1 and .[0].id == $id' "$case_response" >/dev/null
  case_demo_request GET 404 "/api/v1/cases/$litigator_case" "$PARALEGAL_TOKEN"
  case_demo_request PUT 403 "/api/v1/cases/$litigator_case/members/$paralegal_id" \
    "$PARALEGAL_TOKEN"
  case_demo_request DELETE 403 "/api/v1/cases/$owner_case/members/$paralegal_id" \
    "$PARALEGAL_TOKEN"
  case_demo_request DELETE 204 "/api/v1/cases/$owner_case/members/$paralegal_id" \
    "$OWNER_TOKEN"
  case_demo_request DELETE 204 "/api/v1/cases/$owner_case/members/$paralegal_id" \
    "$OWNER_TOKEN"
  case_demo_request GET 404 "/api/v1/cases/$owner_case" "$PARALEGAL_TOKEN"
  case_demo_request GET 200 /api/v1/cases "$PARALEGAL_TOKEN"
  jq -e 'length == 0' "$case_response" >/dev/null

  case_demo_request PUT 204 "/api/v1/cases/$owner_case/members/$client_id" \
    "$OWNER_TOKEN"
  case_demo_request GET 200 "/api/v1/cases/$owner_case" "$client_token"
  jq -e --arg id "$owner_case" '.id == $id' "$case_response" >/dev/null
  case_demo_request GET 200 /api/v1/cases "$client_token"
  jq -e --arg id "$owner_case" \
    'length == 1 and .[0].id == $id' "$case_response" >/dev/null
  case_demo_request GET 404 "/api/v1/cases/$litigator_case" "$client_token"
  case_demo_request POST 403 /api/v1/cases "$client_token" \
    -H 'Content-Type: application/json' \
    --data "$(jq -n '{title: "Denied case", reference: "NUC-DENIED"}')"
  case_demo_request PUT 403 "/api/v1/cases/$litigator_case/members/$client_id" \
    "$client_token"
  case_demo_request DELETE 403 "/api/v1/cases/$owner_case/members/$client_id" \
    "$client_token"
  case_demo_request POST 403 /api/v1/documents "$client_token" \
    -H 'X-Document-Name: document.txt' --data-binary "@$WORK_DIR/document.txt"
  case_demo_request POST 403 "/api/v1/documents/$DOCUMENT_ID/verify" "$client_token"
  case_demo_request POST 403 "/api/v1/documents/$DOCUMENT_ID/seal" "$client_token"
  case_demo_request GET 403 "/api/v1/documents/$DOCUMENT_ID/evidence" "$client_token"
  case_demo_request DELETE 204 "/api/v1/cases/$owner_case/members/$client_id" \
    "$OWNER_TOKEN"
  case_demo_request GET 404 "/api/v1/cases/$owner_case" "$client_token"

  # The same bearer session must observe current account role and active state.
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
    "UPDATE users SET role = 'paralegal' WHERE email = 'litigator-cases@example.com'" \
    >/dev/null
  case_demo_request POST 403 /api/v1/cases "$litigator_token" \
    -H 'Content-Type: application/json' \
    --data "$(jq -n '{title: "Denied case", reference: "NUC-DENIED"}')"
  case_demo_request GET 200 "/api/v1/cases/$litigator_case" "$litigator_token"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
    "UPDATE users SET active = FALSE WHERE email = 'litigator-cases@example.com'" \
    >/dev/null
  case_demo_request GET 401 "/api/v1/cases/$litigator_case" "$litigator_token"

  printf 'Case API demo passed: isolation, assignments, revocation, client restrictions.\n'
}

case_demo
unset -f case_demo case_demo_login case_demo_enroll case_demo_request
