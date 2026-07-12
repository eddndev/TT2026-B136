#!/usr/bin/env bash
# gen-crl.sh - generate the certificate revocation list (CRL).
#
# Produces a PEM encoded CRL from the CA database. The CRL is valid
# for 7 days (default_crl_days in openssl.cnf), so it must be
# regenerated at least weekly and after every revocation made with
# revoke.sh.
#
# The CA working directory is taken from the PKI_CA_DIR environment
# variable and defaults to "pki-ca" under the current working
# directory. Run init-ca.sh first.
#
# Usage:
#   ./gen-crl.sh
#   PKI_CA_DIR=/srv/pki-ca ./gen-crl.sh
#
# The script can be re-run at any time; it overwrites the previous
# CRL with a fresh one.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENSSL_CNF="$SCRIPT_DIR/openssl.cnf"

# Exported so that $ENV::PKI_CA_DIR in openssl.cnf resolves.
export PKI_CA_DIR="${PKI_CA_DIR:-$PWD/pki-ca}"

CRL_FILE="$PKI_CA_DIR/crl/crl.pem"

die() {
    printf 'gen-crl.sh: error: %s\n' "$1" >&2
    exit 1
}

command -v openssl >/dev/null 2>&1 || die "openssl not found in PATH"
[ -f "$OPENSSL_CNF" ] || die "configuration file not found: $OPENSSL_CNF"
[ -f "$PKI_CA_DIR/private/ca.key.pem" ] && [ -f "$PKI_CA_DIR/index.txt" ] \
    || die "CA not initialized in $PKI_CA_DIR (run init-ca.sh first)"

mkdir -p "$PKI_CA_DIR/crl"

openssl ca -config "$OPENSSL_CNF" -gencrl -out "$CRL_FILE"

printf 'CRL generated successfully.\n'
printf '  CRL file: %s\n' "$CRL_FILE"
openssl crl -noout -lastupdate -nextupdate -in "$CRL_FILE" | sed 's/^/  /'
REVOKED_COUNT="$(awk -F '\t' '$1 == "R"' "$PKI_CA_DIR/index.txt" | wc -l)"
printf '  Revoked certificates listed: %s\n' "$REVOKED_COUNT"
