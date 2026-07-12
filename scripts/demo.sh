#!/usr/bin/env bash
#
# demo.sh -- end-to-end walkthrough of the despacho-cli command line.
#
# Drives the full evidence lifecycle: internal certificate authority,
# document hashing, encryption at rest with tamper detection, digital
# signature, trusted timestamp, verification (positive and negative),
# independent verification with openssl, certificate revocation, and the
# authentication and audit-chain features.
#
# File-layout assumptions of this walkthrough (the command handlers are
# expected to follow them):
#   - pki init-ca writes ca.pem and ca.key in the current directory,
#     pki issue writes partner.pem and partner.key, and pki gen-crl
#     writes crl.pem.
#   - vault encrypt, sign, and timestamp write their output next to the
#     input file with the suffixes .enc, .sig, and .tsr respectively.
#   - package export produces a tar.gz archive containing the document,
#     the detached signature, the signer and CA certificates, and the
#     timestamp token.
#
# All working files are created in a temporary directory that is removed
# on exit. Nothing is written inside the repository.

set -euo pipefail

echo "NOTICE: this script exercises the despacho-cli command line end to end."
echo "NOTICE: it requires the built binary on PATH, or cargo available to"
echo "NOTICE: build and run it from this workspace."
echo

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FLIP_BYTE="$SCRIPT_DIR/flip-byte.sh"

# Resolve the command line under test: prefer an installed binary, fall
# back to building and running it with cargo from this workspace.
if command -v despacho-cli > /dev/null 2>&1; then
  CLI=(despacho-cli)
else
  CLI=(cargo run --quiet --manifest-path "$SCRIPT_DIR/../Cargo.toml" \
    --bin despacho-cli --)
fi

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

# Print a command that MUST be rejected, run it, and abort the demo if it
# unexpectedly succeeds. Used for the tamper and revocation checks.
expect_failure() {
  echo "+ (expected to fail) $*"
  if "$@"; then
    echo "FAIL: command succeeded but should have been rejected" >&2
    exit 1
  fi
  echo "OK: command was rejected, as expected"
}

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
cd "$WORK_DIR"

# Fixed document identity so every run of the demo is reproducible.
DOC_ID="00000000-0000-4000-8000-000000000001"

banner "Reproducibility banner"
run openssl version
run "${CLI[@]}" --version

banner "Create the internal CA and issue a partner certificate"
run "${CLI[@]}" pki init-ca
run "${CLI[@]}" pki issue --cn "Demo Partner"
run "${CLI[@]}" pki show partner.pem

banner "Hash the document"
# Sample document, padded so later byte offsets are always in range.
{
  echo "Sample case file used by the demo walkthrough."
  for i in $(seq 1 60); do
    echo "Line $i of filler content to give the document a realistic size."
  done
} > document.txt
run "${CLI[@]}" crypto hash document.txt

banner "Encrypt at rest, tamper with the ciphertext, observe rejection"
# The vault derives per-document keys from the key-encrypting key in
# KEK_BASE64 (see .env.example). A throwaway key is enough for the demo.
KEK_BASE64="$(openssl rand -base64 32)"
export KEK_BASE64
run "${CLI[@]}" vault encrypt document.txt --doc-id "$DOC_ID" --version 1
# Flip one byte in the middle of the stored ciphertext. Authenticated
# encryption must refuse to decrypt the modified package.
run "$FLIP_BYTE" document.txt.enc 512
expect_failure "${CLI[@]}" vault decrypt document.txt.enc --doc-id "$DOC_ID"
# Flipping the same byte again restores the package, so decryption
# succeeds and proves the rejection above was caused by the tampering.
run "$FLIP_BYTE" document.txt.enc 512
run "${CLI[@]}" vault decrypt document.txt.enc --doc-id "$DOC_ID"
# Rotate the key-encrypting key: stored data keys are rewrapped so the
# ciphertext stays readable under the new key from here on.
run "${CLI[@]}" vault rotate-kek

banner "Sign the document and obtain a trusted timestamp"
run "${CLI[@]}" sign document.txt --cert partner.pem --key partner.key
# The mock authority answers locally so the demo runs without network
# access or credentials for the real timestamp service.
run "${CLI[@]}" timestamp document.txt.sig --mock

banner "Full verification, then tamper with the document and re-verify"
run "${CLI[@]}" verify document.txt --sig document.txt.sig \
  --tsr document.txt.sig.tsr --cert partner.pem
# Negative case: one flipped byte in a copy of the document must break
# the signature check while the original still verifies.
cp document.txt tampered.txt
run "$FLIP_BYTE" tampered.txt
expect_failure "${CLI[@]}" verify tampered.txt --sig document.txt.sig \
  --tsr document.txt.sig.tsr --cert partner.pem

banner "Export a verification package and verify it with openssl only"
run "${CLI[@]}" package export --out evidence.tar.gz
mkdir evidence
run tar -xzf evidence.tar.gz -C evidence
run ls -l evidence
# An outside party needs nothing but openssl to check the evidence:
# 1. the signer certificate chains to the CA certificate;
run openssl verify -CAfile evidence/ca.pem evidence/partner.pem
# 2. the detached signature matches the document and the signer key;
run openssl x509 -in evidence/partner.pem -pubkey -noout \
  -out evidence/partner.pub.pem
run openssl dgst -sha256 -verify evidence/partner.pub.pem \
  -signature evidence/document.txt.sig evidence/document.txt
# 3. the timestamp token is well formed and covers the signature.
run openssl ts -reply -in evidence/document.txt.sig.tsr -text

banner "Revoke the partner certificate and re-verify"
run "${CLI[@]}" pki revoke --serial 1
run "${CLI[@]}" pki gen-crl
# With the revocation list published, the certificate no longer passes
# either the openssl chain check or the tool's own verification.
expect_failure openssl verify -crl_check -CAfile ca.pem \
  -CRLfile crl.pem partner.pem
expect_failure "${CLI[@]}" verify document.txt --sig document.txt.sig \
  --tsr document.txt.sig.tsr --cert partner.pem

banner "Authentication: password hashing and one-time passwords"
run "${CLI[@]}" auth calibrate
DEMO_PASSWORD="correct-horse-battery-staple"
# hash-password reads the password from standard input, prints the hash.
echo "+ printf (password) | despacho-cli auth hash-password"
STORED_HASH="$(printf '%s' "$DEMO_PASSWORD" | "${CLI[@]}" auth hash-password)"
echo "stored hash: $STORED_HASH"
# verify-password reads the password on the first line of standard input
# and the stored hash on the second line.
echo "+ printf (password, hash) | despacho-cli auth verify-password"
printf '%s\n%s\n' "$DEMO_PASSWORD" "$STORED_HASH" \
  | "${CLI[@]}" auth verify-password
run "${CLI[@]}" auth totp enroll --user demo@example.com
# A real six-digit code changes every 30 seconds, so the placeholder
# below only demonstrates the command shape; rejection is tolerated.
run "${CLI[@]}" auth totp verify --code 000000 \
  || echo "note: placeholder code rejected (expected without a real device)"

banner "Audit chain"
run "${CLI[@]}" audit append --action demo.hash --resource document.txt
run "${CLI[@]}" audit append --action demo.sign --resource document.txt.sig
run "${CLI[@]}" audit append --action demo.export --resource evidence.tar.gz
run "${CLI[@]}" audit verify-chain
run "${CLI[@]}" audit show

banner "Demo complete"
echo "All stages finished. Working files lived in $WORK_DIR and are"
echo "removed on exit."
