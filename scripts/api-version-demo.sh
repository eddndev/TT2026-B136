#!/usr/bin/env bash
# Version and historical-evidence checks in api-demo.sh's disposable database.

version_demo_request() {
  local method="$1" expected="$2" route="$3" actual
  shift 3
  actual="$(curl -sS -o "$WORK_DIR/version-response.json" -w '%{http_code}' \
    -X "$method" "$BASE_URL$route" -H "Authorization: Bearer $RECOVERY_TOKEN" "$@")"
  if [ "$actual" != "$expected" ]; then
    printf 'api-version-demo.sh: %s %s returned %s, expected %s\n' \
      "$method" "$route" "$actual" "$expected" >&2
    cat "$WORK_DIR/version-response.json" >&2
    return 1
  fi
}

version_demo() {
  local case_id="$1" route="/api/v1/cases/$1/documents/$DOCUMENT_ID"
  local response="$WORK_DIR/version-response.json"
  printf 'A second immutable version of the imported document.\n' >"$WORK_DIR/revised.txt"
  version_demo_request POST 201 "$route/versions?expected_version=1" \
    -H 'X-Document-Name: revised.txt' --data-binary "@$WORK_DIR/revised.txt"
  jq -e --arg id "$DOCUMENT_ID" \
    '.id == $id and .version == 2 and .name == "revised.txt" and .sealed == false' "$response" >/dev/null
  version_demo_request POST 409 "$route/versions?expected_version=1" \
    -H 'X-Document-Name: revised.txt' --data-binary "@$WORK_DIR/revised.txt"
  jq -e '.error.code == "document_version_conflict"' "$response" >/dev/null
  version_demo_request GET 200 "$route"
  jq -e '.version == 2 and .sealed == false' "$response" >/dev/null
  version_demo_request GET 200 "/api/v1/cases/$case_id/documents?sealed=true"
  jq -e --arg id "$DOCUMENT_ID" 'all(.documents[]; .id != $id)' "$response" >/dev/null
  version_demo_request GET 200 "$route/versions?limit=1"
  jq -e '.versions[0].version == 2 and .has_more == true
    and .next_before_version == 2 and .first_available_version == 1' "$response" >/dev/null
  version_demo_request GET 200 "$route/versions?limit=1&before_version=2"
  jq -e '.versions[0].version == 1 and .versions[0].sealed == true
    and .has_more == false and .next_before_version == null' "$response" >/dev/null
  for action in seal verify; do
    version_demo_request POST 409 "$route/$action"
    jq -e '.error.code == "document_version_required"' "$response" >/dev/null
  done
  version_demo_request GET 409 "$route/evidence"
  jq -e '.error.code == "document_version_required"' "$response" >/dev/null
  version_demo_request POST 409 "$route/versions/2/verify"
  jq -e '.error.code == "document_not_sealed"' "$response" >/dev/null
  version_demo_request POST 200 "$route/versions/2/seal"
  jq -e '.version == 2 and .sealed == true' "$response" >/dev/null
  version_demo_request POST 200 "$route/versions/2/verify"
  jq -e --arg id "$DOCUMENT_ID" \
    '.id == $id and .version == 2 and .verdict == "valid"' "$response" >/dev/null
  curl -fsS "$BASE_URL$route/versions/2/evidence" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" -o "$WORK_DIR/version-two-evidence.zip"
  unzip -q "$WORK_DIR/version-two-evidence.zip" -d "$WORK_DIR/version-two-evidence"
  cmp "$WORK_DIR/revised.txt" "$WORK_DIR/version-two-evidence/revised.txt"
  (
    cd "$WORK_DIR/version-two-evidence"
    openssl x509 -in certificado.pem -pubkey -noout -out signer.pub.pem
    openssl dgst -sha256 -verify signer.pub.pem -signature revised.txt.sig revised.txt >/dev/null
    openssl ts -verify -data revised.txt -in revised.txt.tsr -CAfile tsa-chain.pem >/dev/null
  )
  # The same legacy snapshot still reconciles after a new version is appended.
  migration_demo_export "$case_id" "$WORK_DIR/historical-evidence"
  printf 'Version API demo passed: compare-and-append, exact actions, immutable historical ZIP.\n'
}

version_demo_restored() {
  local route="/api/v1/cases/$1/documents/$DOCUMENT_ID"
  version_demo_request GET 200 "$route/versions"
  jq -e '(.versions | map(.version)) == [2,1] and .has_more == false' \
    "$WORK_DIR/version-response.json" >/dev/null
  version_demo_request POST 200 "$route/versions/2/verify"
  jq -e '.version == 2 and .verdict == "valid"' "$WORK_DIR/version-response.json" >/dev/null
  curl -fsS "$BASE_URL$route/versions/2/evidence" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" -o "$WORK_DIR/restored-version-two.zip"
  cmp "$WORK_DIR/version-two-evidence.zip" "$WORK_DIR/restored-version-two.zip"
  printf 'Version restore demo passed: both snapshots and evidence ZIPs preserved.\n'
}
