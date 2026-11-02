#!/usr/bin/env bash
# evidence-verify-contract-fixtures.sh - regenerate the bilateral contract
# test fixtures of crates/evidence-verify from an ephemeral PKI.
#
# Builds a throwaway root that is also the time-stamping root, a Client and
# a firm signer certificate with the internal declaration shape, a
# time-stamping certificate and the CRLs of a contract signature, then the
# request, both certificate bindings, both party statements and their
# signatures, the three time-stamp responses T1, Td and Tf, the contract
# manifest and the application records, with the same OpenSSL
# configuration as the deployment (pki/openssl.cnf and pki/tsa.cnf). Every
# file except fixtures.json, README.md and the variant CRL is written under
# its exact package entry name, so the tests assemble the full and the
# partial package from the directory.
#
# The PDF is larger than 64 KiB. The Client CRL parte-1-crl.pem covers T1;
# the firm CRL parte-2-crl.pem is a later list whose thisUpdate is after the
# T1 interval, so it never covers T1 and the full package needs a CRL per
# token. parte-2-crl-cliente-revocado.pem is that later list again with the
# Client certificate revoked before Tf. The script waits a few seconds
# between T1 and Td for that reason.
#
# The old fixtures directory crates/evidence-verify/tests/fixtures is not
# touched. Private keys live only in a temporary directory that is removed
# on exit. Keys, signatures and time-stamps are random or time dependent, so
# every run produces new bytes; tests read the facts from the entries
# themselves or from fixtures.json.
#
# Usage: scripts/evidence-verify-contract-fixtures.sh
# Requires bash, GNU date, OpenSSL 3.4 or later and python3.
# Rationale: docs/adr/0086-bilateral-contract-signing.md.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
OPENSSL_CNF="$REPO_DIR/pki/openssl.cnf"
TSA_CNF="$REPO_DIR/pki/tsa.cnf"
HELPER="$SCRIPT_DIR/evidence_verify_contract_fixtures.py"
VECTORS="$REPO_DIR/crates/domain/tests/fixtures/contract_statement_vectors.json"
# shellcheck source=scripts/evidence-verify-fixtures-lib.sh
. "$SCRIPT_DIR/evidence-verify-fixtures-lib.sh"
OUT_DIR="$REPO_DIR/crates/evidence-verify/tests/contract-fixtures"

NOT_BEFORE=20260101000000Z
NOT_AFTER=20360101000000Z
SUBJECT_BASE="/C=MX/ST=Ciudad de Mexico/L=Ciudad de Mexico/O=Despacho Juridico Demo"

for tool in openssl python3 date awk sed sha256sum sleep; do
    command -v "$tool" >/dev/null 2>&1 || die "$tool not found in PATH"
done
for file in "$OPENSSL_CNF" "$TSA_CNF" "$HELPER" "$VECTORS"; do
    [ -f "$file" ] || die "file not found: $file"
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
KEYS="$WORK/keys"
STAGE="$WORK/certs"
PACKAGE="$WORK/package"
STATE="$WORK/state.json"
FACTS="$WORK/facts.txt"
LOG="$WORK/command.log"
mkdir -p "$KEYS" "$STAGE" "$PACKAGE"
chmod 700 "$KEYS"
export TSA_DIR="$WORK/root-tsa"
TSA_TEST_CNF="$TSA_CNF"
: > "$FACTS"

helper() {
    python3 -I "$HELPER" --dir "$PACKAGE" --state "$STATE" "$@"
}

# UTCTime and GeneralizedTime texts of a Unix instant.
utc_time() { date -u -d "@$1" +%y%m%d%H%M%SZ; }
general_time() { date -u -d "@$1" +%Y%m%d%H%M%SZ; }

# genTime of the response ENTRY in the package, as Unix seconds.
gen_time() {
    local text
    text="$(openssl ts -reply -in "$PACKAGE/$1" -text 2>"$LOG" | sed -n 's/^Time stamp: //p')"
    [ -n "$text" ] || die "no genTime in $1"
    date -u -d "$text" +%s
}

# sign_entry KEY ENTRY: detached RSA PKCS#1 v1.5 SHA-256 signature of the
# package entry ENTRY into ENTRY.sig with the key of KEY.
sign_entry() {
    quiet openssl dgst -sha256 -sign "$KEYS/$1.key.pem" -out "$PACKAGE/${2%.bin}.sig" \
        "$PACKAGE/$2"
    quiet openssl dgst -sha256 -verify "$WORK/$1.pub.pem" \
        -signature "$PACKAGE/${2%.bin}.sig" "$PACKAGE/$2"
}

# stamp_entry TOKEN COVERED: a response over the SHA-256 of the package
# entry COVERED into the package entry TOKEN, then verified like the
# deployment verifier does.
stamp_entry() {
    local digest
    digest="$(sha256_hex "$PACKAGE/$2")"
    STAGE="$PACKAGE" stamp "$1" internal_tsa "$TSA_DIR" "$digest" response
    quiet openssl ts -verify -digest "$digest" -in "$PACKAGE/$1" -CAfile "$PACKAGE/ca.pem"
}

NOW="$(date -u +%s)"
THIS_UPDATE=$(( NOW / 3600 * 3600 - 3600 ))
NEXT_UPDATE=$(( THIS_UPDATE + 7 * 86400 ))
THIS_UPDATE_TEXT="$(general_time "$THIS_UPDATE")"
NEXT_UPDATE_TEXT="$(general_time "$NEXT_UPDATE")"

printf 'Generating evidence-verify contract fixtures in %s\n' "$OUT_DIR"

ROOT_CA="$WORK/root-ca"
new_root "$ROOT_CA" 3000
quiet openssl x509 -in "$ROOT_CA/ca.crt.pem" -outform DER -out "$STAGE/root.der"
issue "$ROOT_CA" client "Cliente de Prueba" "Clientes" 3072 "" v3_internal_declaration
issue "$ROOT_CA" firm "Firmante del Despacho" "Personal del Despacho" 3072 "" \
    v3_internal_declaration
issue "$ROOT_CA" tsa "TSA Interna Despacho Juridico Demo" "Autoridad de Sellado de Tiempo" \
    3072 "$TSA_CNF" v3_tsa
for name in client firm; do
    quiet openssl x509 -in "$ROOT_CA/certs/$name.crt.pem" -pubkey -noout \
        -out "$WORK/$name.pub.pem"
done
new_tsa "$TSA_DIR" "$ROOT_CA" tsa

# Shared entries and the material of each party.
quiet openssl x509 -inform DER -in "$STAGE/root.der" -out "$PACKAGE/ca.pem"
cp "$PACKAGE/ca.pem" "$PACKAGE/tsa-chain.pem"
cp "$PACKAGE/ca.pem" "$PACKAGE/parte-1-tsa-chain.pem"
gen_crl "$ROOT_CA" before-t1.crl
for position in 1 2; do
    name="$([ "$position" = 1 ] && echo client || echo firm)"
    quiet openssl x509 -inform DER -in "$STAGE/$name.der" \
        -out "$PACKAGE/parte-$position-certificado.pem"
    cp "$STAGE/root.der" "$PACKAGE/parte-$position-vinculo-raiz.der"
    cp "$STAGE/before-t1.crl.der" "$PACKAGE/parte-$position-vinculo-crl.der"
done
cp "$ROOT_CA/crl/before-t1.crl.pem" "$PACKAGE/parte-1-crl.pem"

# The request, both bindings and the Client statement, signed by their
# holders, then T1 over the Client signature.
helper document
helper begin --vectors "$VECTORS" --root-der "$STAGE/root.der" \
    --client-der "$STAGE/client.der" --firm-der "$STAGE/firm.der" \
    --created-at "$(( NOW - 600 ))" --expires-at "$(( NOW + 7 * 86400 ))" \
    --client-registered-at "$(( THIS_UPDATE + 60 ))" \
    --firm-registered-at "$(( THIS_UPDATE + 120 ))"
sign_entry client parte-1-vinculo.bin
sign_entry firm parte-2-vinculo.bin
sign_entry client parte-1-declaracion.bin
stamp_entry parte-1-declaracion.sig.tsr parte-1-declaracion.sig
T1="$(gen_time parte-1-declaracion.sig.tsr)"

# The renewed list starts after the T1 interval (accuracy one second) and
# Td must start after it, so wait until four seconds past T1.
RENEWED=$(( T1 + 2 ))
while [ "$(date -u +%s)" -lt $(( T1 + 4 )) ]; do sleep 1; done
gen_crl "$ROOT_CA" after-t1.crl "$(general_time "$RENEWED")" \
    "$(general_time $(( RENEWED + 7 * 86400 )))"
cp "$ROOT_CA/crl/after-t1.crl.pem" "$PACKAGE/parte-2-crl.pem"
REVOKED_AT_TEXT="$(utc_time "$RENEWED")"
revoke client
gen_crl "$ROOT_CA" client-revoked.crl "$(general_time "$RENEWED")" \
    "$(general_time $(( RENEWED + 7 * 86400 )))"

# The firm statement over S1 and T1, then Td over the PDF, the manifest
# and Tf over the manifest.
helper counter
sign_entry firm parte-2-declaracion.bin
stamp_entry contrato.pdf.tsr contrato.pdf
TD="$(gen_time contrato.pdf.tsr)"
helper manifest
stamp_entry contrato-manifiesto.json.tsr contrato-manifiesto.json
TF="$(gen_time contrato-manifiesto.json.tsr)"
helper records --client-accepted-at "$T1" --firm-accepted-at "$TF" \
    --completed-at "$TF" --exported-at "$(( TF + 1 ))"
[ "$T1" -le "$TD" ] && [ "$TD" -le "$TF" ] || die "tokens out of order: $T1 $TD $TF"
[ $(( TD - 1 )) -ge "$RENEWED" ] || die "Td starts before the renewed CRL"

# Sanity checks with OpenSSL: each certificate chains to the root and is
# not revoked in the list of its instant, and the variant revokes the
# Client.
for position in 1 2; do
    quiet openssl verify -x509_strict -crl_check -CAfile "$PACKAGE/ca.pem" \
        -CRLfile "$PACKAGE/parte-$position-crl.pem" \
        "$PACKAGE/parte-$position-certificado.pem"
done
quiet openssl x509 -inform DER -in "$STAGE/tsa.der" -out "$WORK/tsa.pem"
quiet openssl verify -x509_strict -crl_check -CAfile "$PACKAGE/ca.pem" \
    -CRLfile "$PACKAGE/parte-2-crl.pem" "$WORK/tsa.pem"
must_fail openssl verify -x509_strict -crl_check -CAfile "$PACKAGE/ca.pem" \
    -CRLfile "$ROOT_CA/crl/client-revoked.crl.pem" "$PACKAGE/parte-1-certificado.pem"
cp "$ROOT_CA/crl/client-revoked.crl.pem" "$PACKAGE/parte-2-crl-cliente-revocado.pem"

helper index --generated-at "$NOW" --t1 "$T1" --td "$TD" --tf "$TF" \
    --renewed-this-update "$RENEWED"

rm -rf "$KEYS" "$ROOT_CA/private" "$TSA_DIR/private"
if grep -rlq 'PRIVATE KEY' "$PACKAGE"; then
    die "private key material found in the staged fixtures"
fi

mkdir -p "$OUT_DIR"
find "$OUT_DIR" -mindepth 1 -maxdepth 1 ! -name README.md -exec rm -rf {} +
cp "$PACKAGE"/* "$OUT_DIR"/
printf 'Wrote %s files to %s\n' "$(find "$PACKAGE" -type f | wc -l)" "$OUT_DIR"
