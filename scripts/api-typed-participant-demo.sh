#!/usr/bin/env bash
# Extend api-demo.sh's disposable service and backup campaign with signed participants.

typed_participant_demo_python() {
  TT_TYPED_API_BASE_URL="$BASE_URL" TT_TYPED_API_TOKEN="$RECOVERY_TOKEN" \
    TT_TYPED_API_WORK_DIR="$WORK_DIR" TT_TYPED_API_REPO="$REPO_ROOT" \
    TT_TYPED_API_CERTIFICATE="$PKI_CA_DIR/certs/api-participant.crt.pem" \
    TT_TYPED_API_PRIVATE_KEY="$WORK_DIR/participant-client/private.key.pem" \
    python3 "$REPO_ROOT/scripts/api-typed-participant-demo.py" "$1"
}

typed_participant_demo() {
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue --cn 'API Participant' \
    --purpose participant-declaration >/dev/null
  mkdir -m 700 "$WORK_DIR/participant-client"
  mv "$PKI_CA_DIR/private/api-participant.key.pem" "$WORK_DIR/participant-client/private.key.pem"
  "$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl >/dev/null
  DATABASE_URL="$1" "$CLI" credential-trust publish \
    --root-cert "$CA" --crl "$CRL" --expected-revision 0 >/dev/null
  typed_participant_demo_python capture
}

typed_participant_demo_restored() {
  typed_participant_demo_python restore
}
