#!/usr/bin/env bash
# Reuse the API campaign's isolated CA, database, HTTP runtime and restore.

individual_signing_demo_python() {
  TT_FACT_API_BASE_URL="$BASE_URL" TT_FACT_API_TOKEN="$RECOVERY_TOKEN" \
    TT_FACT_API_WORK_DIR="$WORK_DIR" TT_FACT_API_REPO="$REPO_ROOT" \
    TT_INDIVIDUAL_DATABASE="$2" \
    TT_INDIVIDUAL_CERTIFICATE="$PKI_CA_DIR/certs/individual-litigator.crt.pem" \
    TT_INDIVIDUAL_KEY="$WORK_DIR/individual-client/private.key.pem" \
    python3 -B "$REPO_ROOT/scripts/api-individual-signing-demo.py" "$1"
}

individual_signing_demo() {
  local database="$1" expected serial
  mkdir -m 700 "$WORK_DIR/individual-client"
  (
    umask 077
    openssl req -new -newkey rsa:3072 -nodes -sha256 \
      -subj '/C=MX/O=Despacho Juridico Demo/CN=API Individual Litigator' \
      -keyout "$WORK_DIR/individual-client/private.key.pem" \
      -out "$WORK_DIR/individual-client/public.csr.pem" >/dev/null 2>&1
  )
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue-request \
    --csr "$WORK_DIR/individual-client/public.csr.pem" --name individual-litigator >/dev/null
  test ! -e "$PKI_CA_DIR/private/individual-litigator.key.pem"
  individual_signing_demo_python capture "$database"
  expected="$(jq -er '.prepared.trust_revision' "$WORK_DIR/individual-signing-api-state.json")"
  serial="$(openssl x509 -in "$PKI_CA_DIR/certs/individual-litigator.crt.pem" -noout -serial)"
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" revoke --serial "${serial#serial=}" >/dev/null
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl >/dev/null
  DATABASE_URL="$database" "$CLI" --json credential-trust publish \
    --root-cert "$CA" --crl "$CRL" --expected-revision "$expected" \
    >"$WORK_DIR/individual-signing-trust.json"
  individual_signing_demo_python rotated "$database"
}

individual_signing_demo_restored() {
  individual_signing_demo_python restore "$1"
}
