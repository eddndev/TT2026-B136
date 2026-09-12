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
    cat "$case_response" >&2
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

case_demo_denied_documents() {
  local token="$1" case_id="$2" document_id="$3" expected="$4" seal_expected="$5"
  local collection="/api/v1/cases/$case_id/documents"
  case_demo_request POST "$expected" "$collection" "$token" \
    -H 'X-Document-Name: document.txt' --data-binary "@$WORK_DIR/document.txt"
  case_demo_request POST "$seal_expected" "$collection/$document_id/seal" "$token"
  case_demo_request POST "$expected" "$collection/$document_id/verify" "$token"
  case_demo_request GET "$expected" "$collection/$document_id/evidence" "$token"
}

case_demo_verify_evidence() {
  local token="$1" case_id="$2" document_id="$3" name="$4" destination="$5"
  curl -fsS "$BASE_URL/api/v1/cases/$case_id/documents/$document_id/evidence" \
    -H "Authorization: Bearer $token" -o "$destination.zip"
  unzip -t "$destination.zip" >/dev/null
  unzip -q "$destination.zip" -d "$destination"
  cmp "$WORK_DIR/$name" "$destination/$name"
  (
    cd "$destination"
    openssl x509 -in certificado.pem -pubkey -noout -out signer.pub.pem
    openssl dgst -sha256 -verify signer.pub.pem -signature "$name.sig" "$name" >/dev/null
    cat ca.pem crl.pem >ca-and-crl.pem
    openssl verify -crl_check -CAfile ca-and-crl.pem certificado.pem >/dev/null
    openssl ts -verify -data "$name" -in "$name.tsr" -CAfile tsa-chain.pem >/dev/null
  )
}

case_demo() {
  local case_response="$WORK_DIR/case-response.json"
  local litigator client litigator_token client_token paralegal_id client_id
  local owner_case litigator_case litigator_id other_document hidden_error
  local first_page_id second_page_id plaintext_count
  local missing_case="00000000-0000-4000-8000-000000000000"

  litigator="$(case_demo_enroll litigator-cases@example.com litigator)"
  client="$(case_demo_enroll client-cases@example.com client)"
  litigator_token="$(case_demo_login "$litigator")"
  client_token="$(case_demo_login "$client")"
  paralegal_id="$(jq -er '.user.id' <<<"$PARALEGAL")"
  client_id="$(jq -er '.user.id' <<<"$client")"
  litigator_id="$(jq -er '.user.id' <<<"$litigator")"

  case_demo_request POST 201 /api/v1/cases "$OWNER_TOKEN" \
    -H 'Content-Type: application/json' \
    --data '{"title":"  Owner case  ","reference":"  NUC-DEMO-1  "}'
  owner_case="$(jq -er '.id' "$case_response")"
  jq -e '.title == "Owner case" and .reference == "NUC-DEMO-1"' "$case_response" >/dev/null
  case_demo_request POST 201 /api/v1/cases "$litigator_token" \
    -H 'Content-Type: application/json' \
    --data '{"title":"Litigator case","reference":"NUC-DEMO-2"}'
  litigator_case="$(jq -er '.id' "$case_response")"
  case_demo_request GET 200 "/api/v1/cases/$litigator_case" "$litigator_token"
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=0' "$litigator_token"
  jq -e --arg id "$litigator_case" 'length == 1 and .[0].id == $id' "$case_response" >/dev/null
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=1' "$litigator_token"
  jq -e 'length == 0' "$case_response" >/dev/null
  case_demo_request GET 404 "/api/v1/cases/$owner_case" "$litigator_token"
  hidden_error="$(jq -Sc . "$case_response")"
  case_demo_request GET 404 "/api/v1/cases/$missing_case" "$litigator_token"
  [ "$(jq -Sc . "$case_response")" = "$hidden_error" ]
  case_demo_request GET 200 /api/v1/cases "$OWNER_TOKEN"
  jq -e --arg first "$owner_case" --arg second "$litigator_case" \
    'length == 2 and ([.[].id] | sort) == ([$first, $second] | sort)' "$case_response" >/dev/null
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=0' "$OWNER_TOKEN"
  first_page_id="$(jq -er '.[0].id' "$case_response")"
  case_demo_request GET 200 '/api/v1/cases?limit=1&offset=1' "$OWNER_TOKEN"
  second_page_id="$(jq -er '.[0].id' "$case_response")"
  [ "$first_page_id" != "$second_page_id" ]

  case_demo_request POST 201 "/api/v1/cases/$owner_case/documents" "$OWNER_TOKEN" \
    -H 'X-Document-Name: document.txt' --data-binary "@$WORK_DIR/document.txt"
  DOCUMENT_ID="$(jq -er '.id' "$case_response")"
  jq -e --arg id "$owner_case" '.case_id == $id and .version == 1 and .sealed == false' \
    "$case_response" >/dev/null
  case_demo_request POST 200 "/api/v1/cases/$owner_case/documents/$DOCUMENT_ID/seal" "$OWNER_TOKEN"
  jq -e '.sealed == true' "$case_response" >/dev/null
  case_demo_request POST 200 "/api/v1/cases/$owner_case/documents/$DOCUMENT_ID/verify" "$OWNER_TOKEN"
  jq -e '.verdict == "valid" and .integrity.status == "passed" and .signature.status == "passed"
    and .certificate.status == "passed" and .timestamp.status == "passed"' "$case_response" >/dev/null
  case_demo_verify_evidence "$OWNER_TOKEN" "$owner_case" "$DOCUMENT_ID" document.txt "$WORK_DIR/owner-evidence"

  printf 'Second case with independent document evidence.\n' >"$WORK_DIR/other.txt"
  case_demo_request POST 201 "/api/v1/cases/$litigator_case/documents" "$litigator_token" \
    -H 'X-Document-Name: other.txt' --data-binary "@$WORK_DIR/other.txt"
  other_document="$(jq -er '.id' "$case_response")"
  case_demo_request POST 200 "/api/v1/cases/$litigator_case/documents/$other_document/seal" "$litigator_token"
  case_demo_request POST 200 "/api/v1/cases/$litigator_case/documents/$other_document/verify" "$OWNER_TOKEN"
  case_demo_verify_evidence "$litigator_token" "$litigator_case" "$other_document" other.txt "$WORK_DIR/litigator-evidence"
  case_demo_denied_documents "$litigator_token" "$owner_case" "$DOCUMENT_ID" 404 404
  for operation in seal verify; do
    case_demo_request POST 404 "/api/v1/cases/$litigator_case/documents/$DOCUMENT_ID/$operation" "$OWNER_TOKEN"
  done
  case_demo_request GET 404 "/api/v1/cases/$litigator_case/documents/$DOCUMENT_ID/evidence" "$OWNER_TOKEN"

  case_demo_request POST 403 /api/v1/cases "$PARALEGAL_TOKEN" \
    -H 'Content-Type: application/json' --data '{"title":"Denied","reference":"DENIED"}'
  case_demo_request GET 200 /api/v1/cases "$PARALEGAL_TOKEN"
  jq -e 'length == 0' "$case_response" >/dev/null
  for _ in 1 2; do
    case_demo_request PUT 204 "/api/v1/cases/$owner_case/members/$paralegal_id" "$OWNER_TOKEN"
  done
  case_demo_request GET 200 /api/v1/cases "$PARALEGAL_TOKEN"
  jq -e --arg id "$owner_case" 'length == 1 and .[0].id == $id' "$case_response" >/dev/null
  case_demo_request POST 201 "/api/v1/cases/$owner_case/documents" "$PARALEGAL_TOKEN" \
    -H 'X-Document-Name: paralegal.txt' --data-binary "@$WORK_DIR/document.txt"
  case_demo_request POST 403 "/api/v1/cases/$owner_case/documents/$DOCUMENT_ID/seal" "$PARALEGAL_TOKEN"
  case_demo_request POST 200 "/api/v1/cases/$owner_case/documents/$DOCUMENT_ID/verify" "$PARALEGAL_TOKEN"
  case_demo_verify_evidence "$PARALEGAL_TOKEN" "$owner_case" "$DOCUMENT_ID" document.txt "$WORK_DIR/paralegal-evidence"
  cmp "$WORK_DIR/owner-evidence.zip" "$WORK_DIR/paralegal-evidence.zip"
  case_demo_denied_documents "$PARALEGAL_TOKEN" "$litigator_case" "$other_document" 404 403
  case_demo_request PUT 403 "/api/v1/cases/$litigator_case/members/$paralegal_id" "$PARALEGAL_TOKEN"
  case_demo_request DELETE 403 "/api/v1/cases/$owner_case/members/$paralegal_id" "$PARALEGAL_TOKEN"
  for _ in 1 2; do
    case_demo_request DELETE 204 "/api/v1/cases/$owner_case/members/$paralegal_id" "$OWNER_TOKEN"
  done
  case_demo_request GET 404 "/api/v1/cases/$owner_case" "$PARALEGAL_TOKEN"
  case_demo_request GET 200 /api/v1/cases "$PARALEGAL_TOKEN"
  jq -e 'length == 0' "$case_response" >/dev/null
  case_demo_denied_documents "$PARALEGAL_TOKEN" "$owner_case" "$DOCUMENT_ID" 404 403

  case_demo_request PUT 204 "/api/v1/cases/$owner_case/members/$client_id" "$OWNER_TOKEN"
  case_demo_request GET 200 "/api/v1/cases/$owner_case" "$client_token"
  case_demo_request POST 403 /api/v1/cases "$client_token" \
    -H 'Content-Type: application/json' --data '{"title":"Denied","reference":"DENIED"}'
  case_demo_request PUT 403 "/api/v1/cases/$litigator_case/members/$client_id" "$client_token"
  case_demo_request DELETE 403 "/api/v1/cases/$owner_case/members/$client_id" "$client_token"
  case_demo_denied_documents "$client_token" "$owner_case" "$DOCUMENT_ID" 403 403
  case_demo_denied_documents "$client_token" "$litigator_case" "$other_document" 403 403
  case_demo_request DELETE 204 "/api/v1/cases/$owner_case/members/$client_id" "$OWNER_TOKEN"
  case_demo_request GET 404 "/api/v1/cases/$owner_case" "$client_token"

  for token in "$OWNER_TOKEN" "$litigator_token" "$PARALEGAL_TOKEN" "$client_token"; do
    case_demo_request POST 404 /api/v1/documents "$token" \
      -H "X-Case-Id: $owner_case" -H 'X-Actor: owner@example.com' \
      -H 'X-Document-Name: document.txt' --data-binary "@$WORK_DIR/document.txt"
    case_demo_request POST 404 "/api/v1/documents/$DOCUMENT_ID/seal" "$token"
    case_demo_request POST 404 "/api/v1/documents/$DOCUMENT_ID/verify" "$token"
    case_demo_request GET 404 "/api/v1/documents/$DOCUMENT_ID/evidence" "$token"
  done

  # A removed creator loses document access without changing their bearer token.
  case_demo_request DELETE 204 "/api/v1/cases/$litigator_case/members/$litigator_id" "$OWNER_TOKEN"
  case_demo_denied_documents "$litigator_token" "$litigator_case" "$other_document" 404 404
  case_demo_request PUT 204 "/api/v1/cases/$litigator_case/members/$litigator_id" "$OWNER_TOKEN"
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -c \
    "UPDATE users SET role = 'paralegal' WHERE email = 'litigator-cases@example.com'" >/dev/null
  case_demo_request POST 403 /api/v1/cases "$litigator_token" \
    -H 'Content-Type: application/json' --data '{"title":"Denied","reference":"DENIED"}'
  case_demo_request POST 403 "/api/v1/cases/$litigator_case/documents/$other_document/seal" "$litigator_token"
  case_demo_request POST 200 "/api/v1/cases/$litigator_case/documents/$other_document/verify" "$litigator_token"
  psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -c \
    "UPDATE users SET active = FALSE WHERE email = 'litigator-cases@example.com'" >/dev/null
  case_demo_request POST 401 "/api/v1/cases/$litigator_case/documents/$other_document/verify" "$litigator_token"

  plaintext_count="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
    "SELECT COUNT(*) FROM documents WHERE position(convert_to('Expediente API local verificable.', 'UTF8') in vault) > 0")"
  [ "$plaintext_count" = 0 ]
  case_demo_request POST 409 "/api/v1/cases/$owner_case/documents/$DOCUMENT_ID/seal" "$OWNER_TOKEN"
  case_demo_verify_evidence "$OWNER_TOKEN" "$owner_case" "$DOCUMENT_ID" document.txt "$WORK_DIR/final-owner-evidence"
  cmp "$WORK_DIR/owner-evidence.zip" "$WORK_DIR/final-owner-evidence.zip"
  printf 'Case document API demo passed: four roles, two cases, revocation, preserved evidence.\n'
}

case_demo
unset -f case_demo case_demo_login case_demo_enroll case_demo_request
unset -f case_demo_denied_documents case_demo_verify_evidence
