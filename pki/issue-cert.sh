#!/usr/bin/env bash
# issue-cert.sh - issue an end-entity certificate signed by the CA.
#
# Generates an RSA 3072-bit private key and a certificate signing
# request (CSR) for the given common name, then signs the request
# with the CA for 1 year (365 days) using the v3_end_entity
# extensions from openssl.cnf (digital signature, non repudiation,
# client authentication and email protection).
#
# The CA working directory is taken from the PKI_CA_DIR environment
# variable and defaults to "pki-ca" under the current working
# directory. Run init-ca.sh first. Generated keys live only inside
# that directory and must never be committed to version control.
#
# Usage:
#   ./issue-cert.sh COMMON_NAME
#   PKI_CA_DIR=/srv/pki-ca ./issue-cert.sh "Juan Perez"

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENSSL_CNF="$SCRIPT_DIR/openssl.cnf"

# Exported so that $ENV::PKI_CA_DIR in openssl.cnf resolves.
export PKI_CA_DIR="${PKI_CA_DIR:-$PWD/pki-ca}"

CERT_DAYS=365
KEY_BITS=3072

die() {
    printf 'issue-cert.sh: error: %s\n' "$1" >&2
    exit 1
}

usage() {
    printf 'usage: issue-cert.sh COMMON_NAME\n' >&2
    printf 'example: issue-cert.sh "Juan Perez"\n' >&2
    exit 1
}

[ "$#" -eq 1 ] || usage
CN="$1"
[ -n "$CN" ] || usage

# Restrict the common name to a safe ASCII subset so it can be
# embedded in the -subj argument and in file names without escaping.
if ! printf '%s' "$CN" | grep -Eq '^[A-Za-z0-9][A-Za-z0-9 ._@-]*$'; then
    die "common name may only contain ASCII letters, digits, spaces, and . _ @ - (must start with a letter or digit)"
fi

command -v openssl >/dev/null 2>&1 || die "openssl not found in PATH"
[ -f "$OPENSSL_CNF" ] || die "configuration file not found: $OPENSSL_CNF"
[ -f "$PKI_CA_DIR/private/ca.key.pem" ] && [ -f "$PKI_CA_DIR/ca.crt.pem" ] \
    || die "CA not initialized in $PKI_CA_DIR (run init-ca.sh first)"

# Derive a lowercase file name slug from the common name.
SLUG="$(printf '%s' "$CN" | tr '[:upper:]' '[:lower:]' | tr ' ' '-')"

KEY_FILE="$PKI_CA_DIR/private/$SLUG.key.pem"
CSR_FILE="$PKI_CA_DIR/csr/$SLUG.csr.pem"
CERT_FILE="$PKI_CA_DIR/certs/$SLUG.crt.pem"

[ ! -e "$CERT_FILE" ] || die "certificate already exists: $CERT_FILE (move or remove it to re-issue)"
[ ! -e "$KEY_FILE" ]  || die "private key already exists: $KEY_FILE (move or remove it to re-issue)"

# Subject for the end-entity certificate. Country and organization
# must match the CA subject (dn_defaults in openssl.cnf) because the
# CA policy requires them to match.
SUBJECT="/C=MX/ST=Ciudad de Mexico/L=Ciudad de Mexico"
SUBJECT="$SUBJECT/O=Despacho Juridico Demo/OU=Personal del Despacho/CN=$CN"

# Generate the private key with restrictive permissions.
(
    umask 077
    openssl genpkey -quiet -algorithm RSA \
        -pkeyopt "rsa_keygen_bits:$KEY_BITS" -out "$KEY_FILE"
)

# Create the certificate signing request.
openssl req -config "$OPENSSL_CNF" -new -sha256 \
    -key "$KEY_FILE" -subj "$SUBJECT" -out "$CSR_FILE"

# Backdate the start of validity by a few minutes, as public authorities
# commonly do, so the certificate is immediately usable even when the
# issuing and the relying machine disagree slightly on the time (clock
# skew). Without this, a validation performed in the same second as the
# issuance can land before notBefore and report the certificate as not
# yet valid.
START_DATE="$(date -u -d '5 minutes ago' +%Y%m%d%H%M%SZ)"

# Sign the request with the CA using the end-entity extensions.
openssl ca -config "$OPENSSL_CNF" -batch -notext -md sha256 \
    -startdate "$START_DATE" -days "$CERT_DAYS" -extensions v3_end_entity \
    -in "$CSR_FILE" -out "$CERT_FILE"

printf '\nCertificate issued successfully.\n'
printf '  Certificate: %s\n' "$CERT_FILE"
printf '  Private key: %s (keep secret, never commit)\n' "$KEY_FILE"
openssl x509 -noout -subject -serial -dates -in "$CERT_FILE" | sed 's/^/  /'
