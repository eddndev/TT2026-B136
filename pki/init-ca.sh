#!/usr/bin/env bash
# init-ca.sh - create the root certificate authority (CA).
#
# Generates an RSA 3072-bit private key, self-signs a root certificate
# valid for 5 years (1825 days) and initializes the CA database files
# (index.txt, serial, crlnumber) inside the CA working directory.
#
# The CA working directory is taken from the PKI_CA_DIR environment
# variable and defaults to "pki-ca" under the current working
# directory. It is a runtime artifact: it contains private key
# material and must never be committed to version control. Point
# PKI_CA_DIR outside any repository checkout.
#
# Usage:
#   ./init-ca.sh
#   PKI_CA_DIR=/srv/pki-ca ./init-ca.sh
#
# Re-running the script is safe: it refuses to overwrite an existing
# CA key or certificate and only fills in missing database files.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENSSL_CNF="$SCRIPT_DIR/openssl.cnf"

# Exported so that $ENV::PKI_CA_DIR in openssl.cnf resolves.
export PKI_CA_DIR="${PKI_CA_DIR:-$PWD/pki-ca}"

CA_KEY="$PKI_CA_DIR/private/ca.key.pem"
CA_CERT="$PKI_CA_DIR/ca.crt.pem"
CA_DAYS=1825
KEY_BITS=3072

die() {
    printf 'init-ca.sh: error: %s\n' "$1" >&2
    exit 1
}

command -v openssl >/dev/null 2>&1 || die "openssl not found in PATH"
[ -f "$OPENSSL_CNF" ] || die "configuration file not found: $OPENSSL_CNF"

# Create the CA directory layout. mkdir -p is a no-op when it exists.
mkdir -p "$PKI_CA_DIR/certs" "$PKI_CA_DIR/crl" "$PKI_CA_DIR/csr" \
         "$PKI_CA_DIR/newcerts" "$PKI_CA_DIR/private"
chmod 700 "$PKI_CA_DIR/private"

# Initialize the CA database files only if they do not exist yet.
[ -f "$PKI_CA_DIR/index.txt" ] || : > "$PKI_CA_DIR/index.txt"
[ -f "$PKI_CA_DIR/serial" ]    || printf '1000\n' > "$PKI_CA_DIR/serial"
[ -f "$PKI_CA_DIR/crlnumber" ] || printf '1000\n' > "$PKI_CA_DIR/crlnumber"

if [ -f "$CA_KEY" ] && [ -f "$CA_CERT" ]; then
    printf 'CA already initialized at %s (nothing to do)\n' "$PKI_CA_DIR"
    openssl x509 -noout -subject -dates -in "$CA_CERT"
    exit 0
fi
if [ -f "$CA_KEY" ] || [ -f "$CA_CERT" ]; then
    die "inconsistent CA state: exactly one of $CA_KEY / $CA_CERT exists"
fi

# Generate the CA private key with restrictive permissions.
(
    umask 077
    openssl genpkey -quiet -algorithm RSA \
        -pkeyopt "rsa_keygen_bits:$KEY_BITS" -out "$CA_KEY"
)

# Self-sign the root certificate. The subject comes from the
# dn_defaults section of openssl.cnf.
openssl req -config "$OPENSSL_CNF" -new -x509 -sha256 \
    -days "$CA_DAYS" -extensions v3_root_ca \
    -key "$CA_KEY" -out "$CA_CERT"

printf '\nRoot CA created successfully.\n'
printf '  CA directory:   %s\n' "$PKI_CA_DIR"
printf '  Private key:    %s (keep secret, never commit)\n' "$CA_KEY"
printf '  Certificate:    %s\n' "$CA_CERT"
printf '  Database files: index.txt, serial, crlnumber\n'
openssl x509 -noout -subject -issuer -dates -in "$CA_CERT" | sed 's/^/  /'
