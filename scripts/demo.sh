#!/usr/bin/env bash
#
# demo.sh -- end-to-end walkthrough of the despacho-cli command line.
#
# Drives the full evidence lifecycle with the real binary: internal
# certificate authority, document hashing, encryption at rest with
# tamper detection, digital signature, trusted timestamp, the integral
# verification report (positive and negative), evidence package export
# verified with openssl alone, certificate revocation, authentication
# (password calibration and one-time passwords), and the hash-chained
# audit trail with tamper detection.
#
# Run it from the repository root. Requirements: cargo, openssl, unzip,
# and standard shell tools. All working files live in a temporary
# directory that is removed on exit; nothing is written inside the
# repository except the cargo build artifacts.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKI_SCRIPTS="$REPO_ROOT/pki"
FLIP_BYTE="$REPO_ROOT/scripts/flip-byte.sh"

banner() {
  echo
  echo "====================================================================="
  echo "== $1"
  echo "====================================================================="
}

# Print a command, then run it. Any failure aborts the demo (set -e).
run() {
  echo "+ $*"
  "$@"
}

# Print a command that MUST be rejected, run it, and abort the demo if
# it unexpectedly succeeds. Used for every tamper and revocation check.
expect_failure() {
  echo "+ (expected to fail) $*"
  if "$@"; then
    echo "FAIL: command succeeded but should have been rejected" >&2
    exit 1
  fi
  echo "OK: command was rejected, as expected"
}

banner "Build the binary once"
run cargo build --workspace --manifest-path "$REPO_ROOT/Cargo.toml"
CLI="$(cd "${CARGO_TARGET_DIR:-$REPO_ROOT/target}" && pwd)/debug/despacho-cli"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
cd "$WORK_DIR"

# Every runtime artifact lands inside the scratch directory.
export PKI_CA_DIR="$WORK_DIR/pki-ca"
export TSA_DIR="$WORK_DIR/pki-tsa"
export AUDIT_LOG_PATH="$WORK_DIR/audit-log.jsonl"

# Fixed document identity so every run of the demo is reproducible.
DOC_ID="00000000-0000-4000-8000-000000000001"

banner "Reproducibility banner"
run openssl version
run "$CLI" --version

banner "Create the internal CA, the signer, and the timestamp authority"
run "$CLI" pki --scripts-dir "$PKI_SCRIPTS" init-ca
ISSUE_OUTPUT="$("$CLI" pki --scripts-dir "$PKI_SCRIPTS" issue --cn "Socia Demo")"
echo "$ISSUE_OUTPUT"
SERIAL="$(printf '%s\n' "$ISSUE_OUTPUT" | sed -n 's/^issued certificate serial //p')"
CERT="$PKI_CA_DIR/certs/socia-demo.crt.pem"
KEY="$PKI_CA_DIR/private/socia-demo.key.pem"
ROOT_CERT="$PKI_CA_DIR/ca.crt.pem"
CRL="$PKI_CA_DIR/crl/crl.pem"
run bash "$PKI_SCRIPTS/issue-tsa-cert.sh"
run "$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl

banner "Hash the sample document"
{
  echo "Expediente de demostracion del despacho juridico."
  for i in $(seq 1 60); do
    echo "Linea $i de contenido de relleno para dar tamano realista."
  done
} > document.txt
run "$CLI" crypto hash document.txt

banner "Encrypt at rest, corrupt one byte, observe rejection"
# The vault derives per-document keys from the key-encrypting key in
# KEK_BASE64 (see .env.example). A throwaway key is enough for a demo.
KEK_BASE64="$(openssl rand -base64 32)"
export KEK_BASE64
run "$CLI" vault encrypt document.txt --doc-id "$DOC_ID" --version 1
# Authenticated encryption must refuse a ciphertext with one byte off.
run "$FLIP_BYTE" document.txt.enc 512
expect_failure "$CLI" vault decrypt document.txt.enc --doc-id "$DOC_ID" \
  --out rechazado.txt
# Flipping the same byte again restores the package, so decryption now
# succeeds and proves the rejection above was caused by the tampering.
run "$FLIP_BYTE" document.txt.enc 512
run "$CLI" vault decrypt document.txt.enc --doc-id "$DOC_ID" --out intacto.txt
run cmp document.txt intacto.txt
# Rotate the key-encrypting key: the stored data key is rewrapped and
# the package decrypts under the new key only.
NEW_KEK_BASE64="$(openssl rand -base64 32)"
export NEW_KEK_BASE64
run "$CLI" vault rotate-kek --file document.txt.enc
KEK_BASE64="$NEW_KEK_BASE64"
run "$CLI" vault decrypt document.txt.enc --doc-id "$DOC_ID" --out rotado.txt
run cmp document.txt rotado.txt

banner "Sign the document and obtain a trusted timestamp"
run "$CLI" sign document.txt --cert "$CERT" --key "$KEY"
# The local authority answers without network access or credentials.
run "$CLI" timestamp document.txt --mock --pki-dir "$PKI_SCRIPTS"

banner "Integral verification: all components valid"
run "$CLI" verify document.txt --sig document.txt.sig --tsr document.txt.tsr \
  --cert "$CERT" --ca "$ROOT_CERT" --crl "$CRL"

banner "Tamper with a copy of the document: the signature component fails"
cp document.txt alterado.txt
run "$FLIP_BYTE" alterado.txt 100
expect_failure "$CLI" verify alterado.txt --sig document.txt.sig \
  --tsr document.txt.tsr --cert "$CERT" --ca "$ROOT_CERT" --crl "$CRL"

banner "Export the evidence package and verify it with openssl alone"
run "$CLI" package export document.txt --sig document.txt.sig \
  --tsr document.txt.tsr --cert "$CERT" --ca "$ROOT_CERT" --crl "$CRL" \
  --tsa-chain "$TSA_DIR/tsa-chain.pem" --out evidencia.zip
run unzip -o evidencia.zip -d evidencia
echo "+ head of evidencia/INSTRUCCIONES.md"
sed -n '1,8p' evidencia/INSTRUCCIONES.md
cd evidencia
# The four commands below are exactly the ones INSTRUCCIONES.md walks a
# third party through; none of them involve this repository's software.
run openssl dgst -sha256 document.txt
run openssl x509 -in certificado.pem -pubkey -noout -out firmante.pub.pem
run openssl dgst -sha256 -verify firmante.pub.pem \
  -signature document.txt.sig document.txt
cat ca.pem crl.pem > ca-y-crl.pem
run openssl verify -crl_check -CAfile ca-y-crl.pem certificado.pem
run openssl ts -verify -data document.txt -in document.txt.tsr -CAfile ca.pem
cd "$WORK_DIR"

banner "Revoke the certificate: verification reports the revoked status"
run "$CLI" pki --scripts-dir "$PKI_SCRIPTS" revoke --serial "$SERIAL"
run "$CLI" pki --scripts-dir "$PKI_SCRIPTS" gen-crl
expect_failure "$CLI" verify document.txt --sig document.txt.sig \
  --tsr document.txt.tsr --cert "$CERT" --ca "$ROOT_CERT" --crl "$CRL"

banner "Authentication: hashing cost and one-time passwords"
run "$CLI" auth calibrate
run "$CLI" auth totp enroll --user demo@example.com \
  --secret-out totp-secret.b32
# Compute the current RFC 6238 code independently (HMAC-SHA1, 6 digits,
# 30-second steps) with openssl and coreutils, so acceptance below
# proves interoperability rather than a self-check.
totp_code() {
  local secret_file="$1"
  local key_hex step byte i counter="" hmac offset
  key_hex="$(base32 -d "$secret_file" | od -An -tx1 | tr -d ' \n')"
  step=$(( $(date -u +%s) / 30 ))
  for i in 7 6 5 4 3 2 1 0; do
    byte=$(( (step >> (8 * i)) & 255 ))
    counter+="$(printf '\\%03o' "$byte")"
  done
  hmac="$(printf '%b' "$counter" \
    | openssl dgst -sha1 -mac HMAC -macopt "hexkey:$key_hex" -r \
    | cut -d' ' -f1)"
  offset=$(( 16#${hmac:39:1} ))
  printf '%06d' $(( (16#${hmac:$((offset * 2)):8} & 16#7fffffff) % 1000000 ))
}
CODE="$(totp_code totp-secret.b32)"
echo "+ despacho-cli auth totp verify --code (computed with openssl)"
TOTP_RESULT="$("$CLI" auth totp verify --code "$CODE" \
  --secret-file totp-secret.b32)"
echo "$TOTP_RESULT"
[ "$TOTP_RESULT" = "accepted" ]

banner "Audit chain: append, verify, tamper, verify again"
run "$CLI" audit append --action demo.firma --resource document.txt.sig
run "$CLI" audit append --action demo.sello --resource document.txt.tsr
run "$CLI" audit verify-chain
run "$CLI" audit show
# Alter one recorded action in place; the length stays the same, so
# only the hash chain can notice.
run sed -i 's/demo.sello/demo.XXXXX/' "$AUDIT_LOG_PATH"
expect_failure "$CLI" audit verify-chain

banner "Demo complete"
echo "All stages finished. Working files lived in $WORK_DIR and are"
echo "removed on exit."
