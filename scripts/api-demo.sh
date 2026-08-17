#!/usr/bin/env bash
# Automated end-to-end smoke test for the local HTTP document workflow.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKI_SCRIPTS="$REPO_ROOT/pki"

for command in cargo curl jq openssl rg unzip stdbuf; do
  command -v "$command" >/dev/null 2>&1 || {
    printf 'api-demo.sh: required command not found: %s\n' "$command" >&2
    exit 1
  }
done

cargo build --workspace --manifest-path "$REPO_ROOT/Cargo.toml"
CLI="$REPO_ROOT/target/debug/despacho-cli"
WORK_DIR="$(mktemp -d)"
SERVER_PID=""

cleanup() {
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if [ -d "$WORK_DIR" ] && [[ "$WORK_DIR" == /tmp/* ]]; then
    rm -rf -- "$WORK_DIR"
  fi
}
trap cleanup EXIT

export PKI_CA_DIR="$WORK_DIR/pki-ca"
export TSA_DIR="$WORK_DIR/pki-tsa"
export KEK_BASE64
KEK_BASE64="$(openssl rand -base64 32)"
unset CINCEL_BASE_URL CINCEL_API_KEY

"$CLI" pki --scripts-dir "$PKI_SCRIPTS" init-ca >/dev/null
"$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue --cn "API Demo" >/dev/null
bash "$PKI_SCRIPTS/issue-tsa-cert.sh" >/dev/null
"$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl >/dev/null

CERT="$PKI_CA_DIR/certs/api-demo.crt.pem"
KEY="$PKI_CA_DIR/private/api-demo.key.pem"
CA="$PKI_CA_DIR/ca.crt.pem"
CRL="$PKI_CA_DIR/crl/crl.pem"
DATA_DIR="$WORK_DIR/runtime-data"
SERVER_LOG="$WORK_DIR/server.log"

RUST_LOG=warn stdbuf -oL -eL "$CLI" serve \
  --bind 127.0.0.1:0 \
  --data-dir "$DATA_DIR" \
  --signer-cert "$CERT" \
  --signer-key "$KEY" \
  --ca-cert "$CA" \
  --crl "$CRL" \
  --tsa-config "$PKI_SCRIPTS/tsa.cnf" \
  --tsa-dir "$TSA_DIR" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

SERVER_ADDRESS=""
for _ in $(seq 1 100); do
  SERVER_ADDRESS="$(sed -n 's/^listening on http:\/\///p' "$SERVER_LOG" | tail -n 1)"
  if [ -n "$SERVER_ADDRESS" ]; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$SERVER_LOG" >&2
    exit 1
  fi
  sleep 0.1
done
[ -n "$SERVER_ADDRESS" ] || {
  cat "$SERVER_LOG" >&2
  printf 'api-demo.sh: server did not report its address\n' >&2
  exit 1
}
BASE_URL="http://$SERVER_ADDRESS"

curl -fsS "$BASE_URL/healthz" | rg -x 'ok' >/dev/null
printf 'Expediente API local verificable.\n' >"$WORK_DIR/document.txt"

UPLOAD="$(curl -fsS -X POST "$BASE_URL/api/v1/documents" \
  -H 'X-Actor: api-demo' \
  -H 'X-Document-Name: document.txt' \
  --data-binary "@$WORK_DIR/document.txt")"
DOCUMENT_ID="$(jq -er '.id' <<<"$UPLOAD")"
jq -e '.version == 1 and .sealed == false' <<<"$UPLOAD" >/dev/null

if rg -F 'Expediente API local verificable.' "$DATA_DIR/documents" >/dev/null; then
  printf 'api-demo.sh: plaintext leaked into document storage\n' >&2
  exit 1
fi

SEAL="$(curl -fsS -X POST \
  "$BASE_URL/api/v1/documents/$DOCUMENT_ID/seal" \
  -H 'X-Actor: api-demo')"
jq -e '.sealed == true' <<<"$SEAL" >/dev/null

VERIFY="$(curl -fsS -X POST \
  "$BASE_URL/api/v1/documents/$DOCUMENT_ID/verify" \
  -H 'X-Actor: api-demo')"
jq -e '
  .verdict == "valid" and
  .integrity.status == "passed" and
  .signature.status == "passed" and
  .certificate.status == "passed" and
  .timestamp.status == "passed"
' <<<"$VERIFY" >/dev/null

EVIDENCE="$WORK_DIR/evidence.zip"
curl -fsS "$BASE_URL/api/v1/documents/$DOCUMENT_ID/evidence" \
  -H 'X-Actor: api-demo' -o "$EVIDENCE"
unzip -t "$EVIDENCE" >/dev/null
unzip -q "$EVIDENCE" -d "$WORK_DIR/evidence"
cmp "$WORK_DIR/document.txt" "$WORK_DIR/evidence/document.txt"

(
  cd "$WORK_DIR/evidence"
  openssl x509 -in certificado.pem -pubkey -noout -out signer.pub.pem
  openssl dgst -sha256 -verify signer.pub.pem \
    -signature document.txt.sig document.txt >/dev/null
  cat ca.pem crl.pem >ca-and-crl.pem
  openssl verify -crl_check -CAfile ca-and-crl.pem certificado.pem >/dev/null
  openssl ts -verify -data document.txt -in document.txt.tsr \
    -CAfile tsa-chain.pem >/dev/null
)

AUDIT="$(curl -fsS "$BASE_URL/api/v1/audit/verify")"
jq -e '.valid == true and .entries == 4' <<<"$AUDIT" >/dev/null

printf 'API demo passed: %s\n' "$DOCUMENT_ID"
