#!/usr/bin/env bash
# Owner evidence shares the API campaign's database, identity, CA and restore.

owner_certificate_demo_python() {
  TT_OWNER_CERT_BASE="$BASE_URL" TT_OWNER_CERT_TOKEN="$RECOVERY_TOKEN" \
    TT_OWNER_CERT_REVOKED="$OWNER_TOKEN" TT_OWNER_CERT_PARALEGAL="$PARALEGAL_TOKEN" \
    TT_OWNER_CERT_WORK="$WORK_DIR" TT_OWNER_CERT_DATABASE="$2" \
    TT_OWNER_CERT_CERTIFICATE="$PKI_CA_DIR/certs/api-owner-binding.crt.pem" \
    TT_OWNER_CERT_KEY="$WORK_DIR/owner-client/private.key.pem" \
    python3 -B "$REPO_ROOT/scripts/api-owner-certificates-demo.py" "$1"
}

owner_certificate_demo() {
  local database="$1" expected serial
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue --cn 'API Owner Binding' >/dev/null
  mkdir -m 700 "$WORK_DIR/owner-client"
  mv "$PKI_CA_DIR/private/api-owner-binding.key.pem" "$WORK_DIR/owner-client/private.key.pem"
  chmod 600 "$WORK_DIR/owner-client/private.key.pem"
  owner_certificate_demo_python capture "$database"
  expected="$(jq -er '.prepared.trust_revision' "$WORK_DIR/owner-certificate-api-state.json")"
  serial="$(openssl x509 -in "$PKI_CA_DIR/certs/api-owner-binding.crt.pem" -noout -serial)"
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" revoke --serial "${serial#serial=}" >/dev/null
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl >/dev/null
  DATABASE_URL="$database" "$CLI" --json credential-trust publish \
    --root-cert "$CA" --crl "$CRL" --expected-revision "$expected" \
    >"$WORK_DIR/owner-certificate-trust.json"
  owner_certificate_demo_python rotated "$database"
}

owner_certificate_demo_restored() {
  owner_certificate_demo_python restore "$1"
}
