#!/usr/bin/env bash
# issue-tsa-cert.sh - issue the certificate of the local timestamp
# authority (TSA) and prepare its working directory.
#
# Generates an RSA 3072-bit private key and a certificate signing
# request for the TSA, signs the request with the internal CA using
# the v3_tsa extensions from tsa.cnf (critical extendedKeyUsage
# timeStamping, as RFC 3161 requires of a timestamping certificate),
# and prepares the files "openssl ts -reply" reads: the serial file
# and the certificate chain included in responses.
#
# The CA working directory is taken from the PKI_CA_DIR environment
# variable (default "pki-ca" under the current working directory);
# run init-ca.sh first. The TSA working directory is taken from
# TSA_DIR and defaults to "pki-tsa" next to the CA working directory.
# Both directories hold private key material and must never be
# committed to version control.
#
# Usage:
#   ./issue-tsa-cert.sh
#   PKI_CA_DIR=/srv/pki-ca TSA_DIR=/srv/pki-tsa ./issue-tsa-cert.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENSSL_CNF="$SCRIPT_DIR/openssl.cnf"
TSA_CNF="$SCRIPT_DIR/tsa.cnf"

# Exported so that $ENV::PKI_CA_DIR in openssl.cnf and $ENV::TSA_DIR in
# tsa.cnf resolve when openssl parses them.
export PKI_CA_DIR="${PKI_CA_DIR:-$PWD/pki-ca}"
export TSA_DIR="${TSA_DIR:-$(dirname "$PKI_CA_DIR")/pki-tsa}"

CERT_DAYS=365
KEY_BITS=3072

die() {
    printf 'issue-tsa-cert.sh: error: %s\n' "$1" >&2
    exit 1
}

command -v openssl >/dev/null 2>&1 || die "openssl not found in PATH"
[ -f "$OPENSSL_CNF" ] || die "configuration file not found: $OPENSSL_CNF"
[ -f "$TSA_CNF" ] || die "configuration file not found: $TSA_CNF"
[ -f "$PKI_CA_DIR/private/ca.key.pem" ] && [ -f "$PKI_CA_DIR/ca.crt.pem" ] \
    || die "CA not initialized in $PKI_CA_DIR (run init-ca.sh first)"

KEY_FILE="$TSA_DIR/private/tsa.key.pem"
CSR_FILE="$TSA_DIR/tsa.csr.pem"
CERT_FILE="$TSA_DIR/tsa.crt.pem"
CHAIN_FILE="$TSA_DIR/tsa-chain.pem"
SERIAL_FILE="$TSA_DIR/serial"

[ ! -e "$CERT_FILE" ] || die "certificate already exists: $CERT_FILE (move or remove it to re-issue)"
[ ! -e "$KEY_FILE" ]  || die "private key already exists: $KEY_FILE (move or remove it to re-issue)"

mkdir -p "$TSA_DIR/private"
chmod 700 "$TSA_DIR/private"

# Subject for the TSA certificate. Country and organization must match
# the CA subject (dn_defaults in openssl.cnf) because the CA policy
# requires them to match.
SUBJECT="/C=MX/ST=Ciudad de Mexico/L=Ciudad de Mexico"
SUBJECT="$SUBJECT/O=Despacho Juridico Demo/OU=Autoridad de Sellado de Tiempo"
SUBJECT="$SUBJECT/CN=TSA Interna Despacho Juridico Demo"

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
# skew). Without this, a token requested in the same second as the
# issuance can be signed by a certificate that is not yet valid.
START_DATE="$(date -u -d '5 minutes ago' +%Y%m%d%H%M%SZ)"

# Sign the request with the CA. The extensions come from tsa.cnf, not
# from openssl.cnf, so the certificate carries the timestamping
# extendedKeyUsage instead of the end-entity one.
openssl ca -config "$OPENSSL_CNF" -batch -notext -md sha256 \
    -startdate "$START_DATE" -days "$CERT_DAYS" -extfile "$TSA_CNF" -extensions v3_tsa \
    -in "$CSR_FILE" -out "$CERT_FILE"

# Files "openssl ts -reply" reads: the running serial number of issued
# tokens and the chain appended to every response.
[ -f "$SERIAL_FILE" ] || printf '1000\n' > "$SERIAL_FILE"
cp "$PKI_CA_DIR/ca.crt.pem" "$CHAIN_FILE"

printf '\nTSA certificate issued successfully.\n'
printf '  TSA directory: %s\n' "$TSA_DIR"
printf '  Certificate:   %s\n' "$CERT_FILE"
printf '  Private key:   %s (keep secret, never commit)\n' "$KEY_FILE"
printf '  Chain file:    %s\n' "$CHAIN_FILE"
printf '  Serial file:   %s\n' "$SERIAL_FILE"
openssl x509 -noout -subject -serial -dates -in "$CERT_FILE" | sed 's/^/  /'
