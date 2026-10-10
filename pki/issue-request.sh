#!/usr/bin/env bash
# Issue a signing-only leaf for a holder-generated public CSR.
# The CLI validates the exact bounded request with Rust before staging it here.
# No holder key is generated or loaded; only the authority key signs the leaf.
# See docs/adr/0074-individual-document-signatures.md.
#
# Usage: issue-request.sh OUTPUT_NAME PUBLIC_REQUEST_PEM

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENSSL_CNF="$SCRIPT_DIR/openssl.cnf"
export PKI_CA_DIR="${PKI_CA_DIR:-$PWD/pki-ca}"

die() {
    printf 'certificate request issuance failed: %s\n' "$1" >&2
    exit 1
}

[ "$#" -eq 2 ] || die "expected an output name and public request"
NAME="$1"
CSR_FILE="$2"
[[ "$NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ ]] \
    || die "output name must be a safe ASCII identifier"
[ -f "$CSR_FILE" ] || die "public request is unavailable"
[ -f "$OPENSSL_CNF" ] || die "authority configuration is unavailable"
[ -f "$PKI_CA_DIR/private/ca.key.pem" ] && [ -f "$PKI_CA_DIR/ca.crt.pem" ] \
    || die "authority is not initialized"
CERT_FILE="$PKI_CA_DIR/certs/$NAME.crt.pem"
[ ! -e "$CERT_FILE" ] && [ ! -L "$CERT_FILE" ] \
    || die "certificate already exists"
[ ! -e "$PKI_CA_DIR/private/$NAME.key.pem" ] \
    && [ ! -L "$PKI_CA_DIR/private/$NAME.key.pem" ] \
    || die "output name already identifies a private key"

command -v openssl >/dev/null 2>&1 || die "openssl not found in PATH"
openssl req -in "$CSR_FILE" -verify -noout
START_DATE="$(date -u -d '5 minutes ago' +%Y%m%d%H%M%SZ)"
# copy_extensions=none applies the same controlled signing-only profile used
# by internal declarations; statement prefixes separate each application purpose.
openssl ca -config "$OPENSSL_CNF" -batch -notext -md sha256 \
    -startdate "$START_DATE" -days 365 -extensions v3_internal_declaration \
    -in "$CSR_FILE" -out "$CERT_FILE"
printf 'Issued public certificate: %s\n' "$CERT_FILE"
