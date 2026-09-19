#!/usr/bin/env bash
# Race two actual HTTP servers over one document in api-demo.sh's database.

concurrency_demo() {
  local address="" second_url case_id document_id route first_pid second_pid statuses
  local race_dir="$WORK_DIR/concurrent-seal"
  mkdir -p "$race_dir"
  (
    cd "$WORK_DIR"
    RUST_LOG=warn exec stdbuf -oL -eL "$CLI" serve --bind 127.0.0.1:0 \
      --data-dir "$DATA_DIR" --signer-cert "$CERT" --signer-key "$KEY" \
      --ca-cert "$CA" --crl "$CRL" --tsa-config "$PKI_SCRIPTS/tsa.cnf" \
      --tsa-dir "$TSA_DIR"
  ) >"$race_dir/server.log" 2>&1 &
  SECOND_SERVER_PID=$!
  for _ in $(seq 1 100); do
    address="$(sed -n 's/^listening on http:\/\///p' "$race_dir/server.log" | tail -n 1)"
    if [ -n "$address" ]; then break; fi
    kill -0 "$SECOND_SERVER_PID" 2>/dev/null || { cat "$race_dir/server.log" >&2; return 1; }
    sleep 0.1
  done
  [ -n "$address" ] || { cat "$race_dir/server.log" >&2; return 1; }
  second_url="http://$address"
  curl -fsS "$second_url/healthz" | rg -x 'ok' >/dev/null
  case_id="$(psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -Atc \
    "SELECT case_id FROM document_series WHERE id='$DOCUMENT_ID'")"
  document_id="$(curl -fsS -X POST "$BASE_URL/api/v1/cases/$case_id/documents" \
    -H "Authorization: Bearer $RECOVERY_TOKEN" -H 'X-Document-Name: document.txt' \
    --data-binary "@$WORK_DIR/document.txt" | jq -er '.id')"
  route="/api/v1/cases/$case_id/documents/$document_id"

  curl -sS -X POST "$BASE_URL$route/seal" -H "Authorization: Bearer $RECOVERY_TOKEN" \
    -o "$race_dir/first.json" -w '%{http_code}' >"$race_dir/first.status" &
  first_pid=$!
  curl -sS -X POST "$second_url$route/seal" -H "Authorization: Bearer $RECOVERY_TOKEN" \
    -o "$race_dir/second.json" -w '%{http_code}' >"$race_dir/second.status" &
  second_pid=$!
  wait "$first_pid"
  wait "$second_pid"
  statuses="$(cat "$race_dir/first.status" "$race_dir/second.status")"
  case "$statuses" in
    200409|409200) ;;
    *) printf 'api-concurrency-demo.sh: expected one 200 and one 409; received %s\n' "$statuses" >&2
       cat "$race_dir/first.json" "$race_dir/second.json" >&2
       return 1 ;;
  esac
  [ "$(psql "$DATABASE_ADMIN_URL" -v ON_ERROR_STOP=1 -Atc \
    "SELECT COUNT(*) FROM audit_events WHERE action='document.sealed'
      AND resource='case:$case_id:document:$document_id:version:1:sha256:' ||
        (SELECT encode(digest,'hex') FROM documents WHERE id='$document_id' AND version=1)")" -eq 1 ]
  curl -fsS "$BASE_URL$route/evidence" -H "Authorization: Bearer $RECOVERY_TOKEN" \
    -o "$race_dir/first.zip"
  curl -fsS "$second_url$route/evidence" -H "Authorization: Bearer $RECOVERY_TOKEN" \
    -o "$race_dir/second.zip"
  cmp "$race_dir/first.zip" "$race_dir/second.zip"
  unzip -t "$race_dir/first.zip" >/dev/null
  unzip -q "$race_dir/first.zip" -d "$race_dir/evidence"
  cmp "$WORK_DIR/document.txt" "$race_dir/evidence/document.txt"
  (
    cd "$race_dir/evidence"
    openssl x509 -in certificado.pem -pubkey -noout -out signer.pub.pem
    openssl dgst -sha256 -verify signer.pub.pem -signature document.txt.sig document.txt >/dev/null
    cat ca.pem crl.pem >ca-and-crl.pem
    openssl verify -crl_check -CAfile ca-and-crl.pem certificado.pem >/dev/null
    openssl ts -verify -data document.txt -in document.txt.tsr -CAfile tsa-chain.pem >/dev/null
  )
  kill "$SECOND_SERVER_PID"
  wait "$SECOND_SERVER_PID" 2>/dev/null || true
  SECOND_SERVER_PID=""
  printf 'Two-server seal race passed: one seal, one conflict, one audit event, identical valid evidence.\n'
}

concurrency_demo
unset -f concurrency_demo
