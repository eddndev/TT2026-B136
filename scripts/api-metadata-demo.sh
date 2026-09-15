#!/usr/bin/env bash
# Classification and historical-evidence checks in the disposable API cluster.

metadata_demo_request() {
  local method="$1" expected="$2" route="$3" actual
  shift 3
  actual="$(curl -sS -o "$WORK_DIR/metadata-response.json" -w '%{http_code}' \
    -X "$method" "$BASE_URL$route" -H "Authorization: Bearer $RECOVERY_TOKEN" "$@")"
  if [ "$actual" != "$expected" ]; then
    printf 'api-metadata-demo.sh: %s %s returned %s, expected %s\n' \
      "$method" "$route" "$actual" "$expected" >&2
    cat "$WORK_DIR/metadata-response.json" >&2
    return 1
  fi
}

metadata_demo() {
  local case_id="$1" route="/api/v1/cases/$1/documents/$DOCUMENT_ID"
  local response="$WORK_DIR/metadata-response.json" before
  metadata_demo_request GET 200 "$route/metadata"
  jq -e '.metadata_revision == 0 and .document_type == null
    and .classification == null and .tags == []' "$response" >/dev/null
  metadata_demo_request GET 200 "$route/metadata/history"
  jq -e '.revisions == [] and .has_more == false' "$response" >/dev/null
  cat >"$WORK_DIR/classification.json" <<'JSON'
{"expected_metadata_revision":0,"document_type":" Escrito ","classification":" Penal ","tags":["acci\u00f3n","a,b","z","a,b"]}
JSON
  metadata_demo_request PUT 200 "$route/metadata" \
    -H 'Content-Type: application/json' --data-binary "@$WORK_DIR/classification.json"
  jq -e '.metadata_revision == 1 and .document_type == "Escrito"
    and .classification == "Penal" and .tags == ["a,b","acci\u00f3n","z"]' "$response" >/dev/null
  before="$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM audit_events')"
  metadata_demo_request PUT 409 "$route/metadata" \
    -H 'Content-Type: application/json' --data-binary "@$WORK_DIR/classification.json"
  jq -e '.error.code == "document_metadata_conflict"' "$response" >/dev/null
  [ "$(psql "$imported_url" -Atc 'SELECT COUNT(*) FROM audit_events')" = "$before" ]
  metadata_demo_request GET 200 "/api/v1/cases/$case_id/documents" \
    --get --data-urlencode 'document_type=Escrito' --data-urlencode 'classification=Penal' \
    --data-urlencode 'tag=a,b'
  jq -e --arg id "$DOCUMENT_ID" '.documents | length == 1
    and .[0].id == $id and .[0].version == 2 and .[0].current_metadata.metadata_revision == 1' \
    "$response" >/dev/null
  metadata_demo_request PUT 200 "$route/metadata" -H 'Content-Type: application/json' \
    --data-binary '{"expected_metadata_revision":1,"tags":[]}'
  jq -e '.metadata_revision == 2 and .document_type == null
    and .classification == null and .tags == []' "$response" >/dev/null
  metadata_demo_request GET 200 "/api/v1/cases/$case_id/documents?classification=Penal"
  jq -e '.documents == []' "$response" >/dev/null
  metadata_demo_request GET 200 "$route/metadata/history?limit=1"
  jq -e '.revisions[0].metadata_revision == 2 and .has_more == true
    and .next_before_revision == 2' "$response" >/dev/null
  metadata_demo_request GET 200 "$route/metadata/history?limit=1&before_revision=2"
  jq -e '.revisions[0].metadata_revision == 1 and .revisions[0].document_type == "Escrito"
    and .revisions[0].classification == "Penal" and .has_more == false
    and (.revisions[0].changed_by.email | length > 0)
    and (.revisions[0].metadata_digest | length == 64)' "$response" >/dev/null
  metadata_demo_request GET 200 "$route/metadata/history"
  jq -Sc . "$response" >"$WORK_DIR/classification-history.json"

  jq 'del(.expected_metadata_revision)' "$WORK_DIR/classification.json" \
    >"$WORK_DIR/upload-metadata.json"
  metadata_demo_request POST 201 "/api/v1/cases/$case_id/documents/with-metadata" \
    -H 'X-Document-Name: classified.txt' \
    -F "metadata=<$WORK_DIR/upload-metadata.json;type=application/json" \
    -F "file=@$WORK_DIR/document.txt;filename=ignored.txt;type=application/octet-stream"
  jq -e '.version == 1 and .name == "classified.txt" and .sealed == false
    and .current_metadata.metadata_revision == 1
    and .current_metadata.document_type == "Escrito"' "$response" >/dev/null
  METADATA_DOCUMENT_ID="$(jq -r .id "$response")"
  metadata_demo_request GET 200 "/api/v1/cases/$case_id/documents/$METADATA_DOCUMENT_ID/metadata/history"
  jq -Sc . "$response" >"$WORK_DIR/upload-classification-history.json"
  metadata_demo_evidence "$case_id" classified
  printf 'Classification API demo passed: atomic upload, exact filters, stale rejection, immutable history and ZIPs.\n'
}

metadata_demo_evidence() {
  local route="/api/v1/cases/$1/documents/$DOCUMENT_ID" label="$2"
  local version expected
  for version in 1 2; do
    expected="$WORK_DIR/owner-evidence.zip"
    if [ "$version" = 2 ]; then expected="$WORK_DIR/version-two-evidence.zip"; fi
    curl -fsS "$BASE_URL$route/versions/$version/evidence" \
      -H "Authorization: Bearer $RECOVERY_TOKEN" -o "$WORK_DIR/$label-version-$version.zip"
    cmp "$expected" "$WORK_DIR/$label-version-$version.zip"
  done
}

metadata_demo_restored() {
  local route="/api/v1/cases/$1/documents" response="$WORK_DIR/metadata-response.json"
  metadata_demo_request GET 200 "$route/$DOCUMENT_ID/metadata"
  jq -e '.metadata_revision == 2 and .document_type == null
    and .classification == null and .tags == []' "$response" >/dev/null
  metadata_demo_request GET 200 "$route/$DOCUMENT_ID/metadata/history"
  jq -Sc . "$response" >"$WORK_DIR/restored-classification-history.json"
  cmp "$WORK_DIR/classification-history.json" "$WORK_DIR/restored-classification-history.json"
  metadata_demo_request GET 200 "$route/$METADATA_DOCUMENT_ID/metadata/history"
  jq -Sc . "$response" >"$WORK_DIR/restored-upload-classification-history.json"
  cmp "$WORK_DIR/upload-classification-history.json" "$WORK_DIR/restored-upload-classification-history.json"
  metadata_demo_request GET 200 "$route?tag=a%2Cb"
  jq -e --arg id "$METADATA_DOCUMENT_ID" \
    '(.documents | map(.id)) == [$id]' "$response" >/dev/null
  metadata_demo_evidence "$1" restored-classification
  printf 'Classification restore demo passed: revisions, captured authors, exact filters and both historical ZIPs preserved.\n'
}
