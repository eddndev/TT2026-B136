#!/usr/bin/env bash
# revoke.sh - revoke a certificate issued by the CA.
#
# Marks the given certificate as revoked in the CA database
# (index.txt). The revocation only becomes visible to relying
# parties after a new certificate revocation list is generated
# with gen-crl.sh.
#
# The CA working directory is taken from the PKI_CA_DIR environment
# variable and defaults to "pki-ca" under the current working
# directory. Run init-ca.sh first.
#
# Usage:
#   ./revoke.sh CERT_PATH
#   PKI_CA_DIR=/srv/pki-ca ./revoke.sh /srv/pki-ca/certs/juan-perez.crt.pem
#
# Revoking an already revoked certificate is reported and exits
# successfully, so the script can be re-run safely.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENSSL_CNF="$SCRIPT_DIR/openssl.cnf"

# Exported so that $ENV::PKI_CA_DIR in openssl.cnf resolves.
export PKI_CA_DIR="${PKI_CA_DIR:-$PWD/pki-ca}"

die() {
    printf 'revoke.sh: error: %s\n' "$1" >&2
    exit 1
}

usage() {
    printf 'usage: revoke.sh CERT_PATH\n' >&2
    exit 1
}

[ "$#" -eq 1 ] || usage
CERT_PATH="$1"
[ -n "$CERT_PATH" ] || usage

command -v openssl >/dev/null 2>&1 || die "openssl not found in PATH"
[ -f "$OPENSSL_CNF" ] || die "configuration file not found: $OPENSSL_CNF"
[ -f "$CERT_PATH" ] || die "certificate file not found: $CERT_PATH"
[ -f "$PKI_CA_DIR/private/ca.key.pem" ] && [ -f "$PKI_CA_DIR/index.txt" ] \
    || die "CA not initialized in $PKI_CA_DIR (run init-ca.sh first)"

# Read the serial number of the certificate to check its current
# state in the CA database before attempting the revocation.
SERIAL="$(openssl x509 -noout -serial -in "$CERT_PATH" | cut -d= -f2)"
[ -n "$SERIAL" ] || die "could not read serial number from $CERT_PATH"

# index.txt is tab separated: status, expiry, revocation date,
# serial, file name, subject. Status R means already revoked.
if awk -F '\t' -v s="$SERIAL" '$1 == "R" && $4 == s { found = 1 } END { exit !found }' \
    "$PKI_CA_DIR/index.txt"; then
    printf 'Certificate with serial %s is already revoked (nothing to do).\n' "$SERIAL"
    exit 0
fi

openssl ca -config "$OPENSSL_CNF" -revoke "$CERT_PATH"

printf '\nCertificate revoked successfully.\n'
printf '  Certificate: %s\n' "$CERT_PATH"
printf '  Serial:      %s\n' "$SERIAL"
printf 'Run gen-crl.sh to publish an updated revocation list.\n'
