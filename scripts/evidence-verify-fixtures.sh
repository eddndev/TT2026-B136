#!/usr/bin/env bash
# evidence-verify-fixtures.sh - regenerate the public test fixtures of
# crates/evidence-verify from an ephemeral PKI.
#
# Builds a throwaway root, a foreign root, end-entity and time-stamping
# certificates, CRLs, detached signatures, RFC 3161 responses and the
# certificate binding and document consent statements with the same
# OpenSSL configuration as the deployment (pki/openssl.cnf and
# pki/tsa.cnf). Private keys live only in a temporary directory that is
# removed on exit; the fixtures directory receives public artifacts and
# fixtures.json, an index of their digests and of the facts tests need.
#
# Certificates are valid from 2026-01-01T00:00:00Z to
# 2036-01-01T00:00:00Z. Time-stamp tokens carry the generation time as
# genTime, so the CRL windows are placed around it: thisUpdate is the
# start of the generation hour minus one hour, nextUpdate is seven days
# later as gen-crl.sh does, and listed revocations are one day before
# thisUpdate. root-stale.crl instead ends at that thisUpdate, before every
# genTime. Keys and signatures are random, so every run produces new bytes;
# tests read the facts from fixtures.json.
#
# Usage: scripts/evidence-verify-fixtures.sh
# Requires bash, GNU date, OpenSSL 3.4 or later and python3.
# Rationale: docs/adr/0085-in-browser-evidence-validation.md.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
OPENSSL_CNF="$REPO_DIR/pki/openssl.cnf"
TSA_CNF="$REPO_DIR/pki/tsa.cnf"
HELPER="$SCRIPT_DIR/evidence_verify_fixtures.py"
OUT_DIR="$REPO_DIR/crates/evidence-verify/tests/fixtures"

NOT_BEFORE=20260101000000Z
NOT_AFTER=20360101000000Z
POLICY=1.3.6.1.4.1.13762.3
# A second policy that only the test authority section admits; the
# verifier must not admit it.
OTHER_POLICY=1.3.6.1.4.1.13762.4
SUBJECT_BASE="/C=MX/ST=Ciudad de Mexico/L=Ciudad de Mexico/O=Despacho Juridico Demo"
FOREIGN_SUBJECT="$SUBJECT_BASE/OU=Autoridad Certificadora Ajena/CN=CA Raiz Ajena de Prueba"
PERSONNEL_OU="Personal del Despacho"
TSA_OU="Autoridad de Sellado de Tiempo"

die() {
    printf 'evidence-verify-fixtures.sh: error: %s\n' "$1" >&2
    exit 1
}

for tool in openssl python3 date awk sed sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || die "$tool not found in PATH"
done
for file in "$OPENSSL_CNF" "$TSA_CNF" "$HELPER"; do
    [ -f "$file" ] || die "file not found: $file"
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
KEYS="$WORK/keys"
STAGE="$WORK/out"
FACTS="$WORK/facts.txt"
LOG="$WORK/command.log"
mkdir -p "$KEYS" "$STAGE"
chmod 700 "$KEYS"
# pki/tsa.cnf reads TSA_DIR when it is parsed, also as an extension file;
# each time-stamp reply below sets its own authority directory.
export TSA_DIR="$WORK/root-tsa"
: > "$FACTS"

# Run a command, showing its output only when it fails.
quiet() {
    "$@" >"$LOG" 2>&1 || { cat "$LOG" >&2; die "command failed: $*"; }
}

# Run a command that must fail; used for negative sanity checks.
must_fail() {
    ! "$@" >"$LOG" 2>&1 || die "command unexpectedly succeeded: $*"
}

fact() {
    printf '%s\n' "$*" >> "$FACTS"
}

sha256_hex() {
    sha256sum "$1" | awk '{print $1}'
}

NOW="$(date -u +%s)"
THIS_UPDATE=$(( NOW / 3600 * 3600 - 3600 ))
NEXT_UPDATE=$(( THIS_UPDATE + 7 * 86400 ))
REVOKED_AT=$(( THIS_UPDATE - 86400 ))
THIS_UPDATE_TEXT="$(date -u -d "@$THIS_UPDATE" +%Y%m%d%H%M%SZ)"
NEXT_UPDATE_TEXT="$(date -u -d "@$NEXT_UPDATE" +%Y%m%d%H%M%SZ)"
# The CA database stores revocation dates as UTCTime.
REVOKED_AT_TEXT="$(date -u -d "@$REVOKED_AT" +%y%m%d%H%M%SZ)"

# new_root DIR FIRST_SERIAL [SUBJECT]: lay out a CA working directory as
# init-ca.sh does and self-sign its root with the v3_root_ca extensions.
new_root() {
    local dir="$1" first_serial="$2" subject="${3:-}"
    local subject_args=()
    [ -z "$subject" ] || subject_args=(-subj "$subject")
    mkdir -p "$dir/certs" "$dir/crl" "$dir/csr" "$dir/newcerts" "$dir/private"
    chmod 700 "$dir/private"
    : > "$dir/index.txt"
    printf '%s\n' "$first_serial" > "$dir/serial"
    printf '1000\n' > "$dir/crlnumber"
    (
        umask 077
        quiet openssl genpkey -quiet -algorithm RSA \
            -pkeyopt rsa_keygen_bits:3072 -out "$dir/private/ca.key.pem"
    )
    PKI_CA_DIR="$dir" quiet openssl req -config "$OPENSSL_CNF" -new -x509 \
        -sha256 -not_before "$NOT_BEFORE" -not_after "$NOT_AFTER" \
        -extensions v3_root_ca "${subject_args[@]}" \
        -key "$dir/private/ca.key.pem" -out "$dir/ca.crt.pem"
}

# issue CA_DIR NAME COMMON_NAME UNIT BITS EXTFILE SECTION: issue a
# certificate as issue-cert.sh does, with fixed validity. An empty
# EXTFILE takes SECTION from pki/openssl.cnf.
issue() {
    local ca="$1" name="$2" cn="$3" unit="$4" bits="$5" extfile="$6" section="$7"
    local key="$KEYS/$name.key.pem" csr="$ca/csr/$name.csr.pem"
    local ext_args=(-extensions "$section")
    [ -z "$extfile" ] || ext_args=(-extfile "$extfile" -extensions "$section")
    (
        umask 077
        quiet openssl genpkey -quiet -algorithm RSA \
            -pkeyopt "rsa_keygen_bits:$bits" -out "$key"
    )
    PKI_CA_DIR="$ca" quiet openssl req -config "$OPENSSL_CNF" -new -sha256 \
        -key "$key" -subj "$SUBJECT_BASE/OU=$unit/CN=$cn" -out "$csr"
    PKI_CA_DIR="$ca" quiet openssl ca -config "$OPENSSL_CNF" -batch -notext \
        -md sha256 -startdate "$NOT_BEFORE" -enddate "$NOT_AFTER" \
        "${ext_args[@]}" -in "$csr" -out "$ca/certs/$name.crt.pem"
    quiet openssl x509 -in "$ca/certs/$name.crt.pem" -outform DER \
        -out "$STAGE/$name.der"
}

# gen_crl CA_DIR NAME [FROM UNTIL]: sign a CRL over the CA database with
# the fixed window or the given one, using crl_ext and the crlnumber file
# as gen-crl.sh does.
gen_crl() {
    local ca="$1" name="$2" from="${3:-$THIS_UPDATE_TEXT}" until="${4:-$NEXT_UPDATE_TEXT}"
    PKI_CA_DIR="$ca" quiet openssl ca -config "$OPENSSL_CNF" -gencrl \
        -crl_lastupdate "$from" -crl_nextupdate "$until" \
        -out "$ca/crl/$name.pem"
    quiet openssl crl -in "$ca/crl/$name.pem" -outform DER -out "$STAGE/$name.der"
}

# new_tsa DIR CA_DIR NAME: prepare a TSA working directory as
# issue-tsa-cert.sh does, with the issuing root as the appended chain.
new_tsa() {
    local dir="$1" ca="$2" name="$3"
    mkdir -p "$dir/private"
    chmod 700 "$dir/private"
    cp "$KEYS/$name.key.pem" "$dir/private/tsa.key.pem"
    cp "$ca/certs/$name.crt.pem" "$dir/tsa.crt.pem"
    cp "$ca/ca.crt.pem" "$dir/tsa-chain.pem"
    printf '1000\n' > "$dir/serial"
}

# stamp NAME SECTION TSA_DIR DIGEST FORM [POLICY]: request a token over
# DIGEST as the deployment adapter does (SHA-256, -cert, nonce) and
# write a full response, or the bare token when FORM is "token".
stamp() {
    local name="$1" section="$2" tsa="$3" digest="$4" form="$5" policy="${6:-}"
    local query="$WORK/$name.tsq"
    local query_args=(-digest "$digest" -sha256 -cert)
    local reply_args=()
    [ -z "$policy" ] || query_args+=(-tspolicy "$policy")
    [ "$form" = response ] || reply_args=(-token_out)
    quiet openssl ts -query "${query_args[@]}" -out "$query"
    TSA_DIR="$tsa" quiet openssl ts -reply -config "$TSA_TEST_CNF" \
        -section "$section" -queryfile "$query" "${reply_args[@]}" \
        -out "$STAGE/$name"
}

# check_token NAME ANCHOR_PEM DIGEST FORM: the deployment verifier call.
check_token() {
    local name="$1" anchor="$2" digest="$3" form="$4"
    local in_args=()
    [ "$form" = response ] || in_args=(-token_in)
    openssl ts -verify -digest "$digest" -in "$STAGE/$name" "${in_args[@]}" \
        -CAfile "$anchor"
}

sign() {
    quiet openssl dgst -sha256 -sign "$KEYS/$1.key.pem" -out "$STAGE/$2" "$STAGE/$3"
    quiet openssl dgst -sha256 -verify "$WORK/$1.pub.pem" -signature "$STAGE/$2" \
        "$STAGE/$3"
    fact sig "$2" "$1.der" "$3"
}

printf 'Generating evidence-verify fixtures in %s\n' "$OUT_DIR"

# Roots. The root keeps the deployment subject from pki/openssl.cnf.
ROOT_CA="$WORK/root-ca"
FOREIGN_CA="$WORK/foreign-ca"
new_root "$ROOT_CA" 1000
new_root "$FOREIGN_CA" 2000 "$FOREIGN_SUBJECT"
quiet openssl x509 -in "$ROOT_CA/ca.crt.pem" -outform DER -out "$STAGE/root.der"
quiet openssl x509 -in "$FOREIGN_CA/ca.crt.pem" -outform DER \
    -out "$STAGE/foreign-root.der"
fact cert root.der root - -
fact cert foreign-root.der root - -

# Extension sections that deliberately break one rule of a strict
# profile each, plus the production v3_tsa section from pki/tsa.cnf.
BAD_EXT="$WORK/bad-profiles.cnf"
cat > "$BAD_EXT" <<'EOF'
[ tsa_noncritical_eku ]
basicConstraints        = critical, CA:false
keyUsage                = critical, digitalSignature
extendedKeyUsage        = timeStamping
authorityKeyIdentifier  = keyid:always
subjectKeyIdentifier    = hash

[ signer_ca_true ]
basicConstraints        = critical, CA:true
keyUsage                = critical, digitalSignature, nonRepudiation
extendedKeyUsage        = clientAuth, emailProtection
authorityKeyIdentifier  = keyid:always
subjectKeyIdentifier    = hash

[ holder_with_eku ]
basicConstraints        = critical, CA:false
keyUsage                = critical, digitalSignature, nonRepudiation
extendedKeyUsage        = emailProtection
authorityKeyIdentifier  = keyid:always
subjectKeyIdentifier    = hash
EOF

issue "$ROOT_CA" server-signer "Server Seal Signer" "$PERSONNEL_OU" 3072 "" v3_end_entity
issue "$ROOT_CA" holder "Holder Declaration" "$PERSONNEL_OU" 3072 "" v3_internal_declaration
issue "$ROOT_CA" revoked-holder "Revoked Holder Declaration" "$PERSONNEL_OU" 3072 "" \
    v3_internal_declaration
issue "$ROOT_CA" tsa "TSA Interna Despacho Juridico Demo" "$TSA_OU" 3072 "$TSA_CNF" v3_tsa
issue "$ROOT_CA" tsa-noncritical-eku "TSA Non Critical EKU" "$TSA_OU" 3072 "$BAD_EXT" \
    tsa_noncritical_eku
issue "$ROOT_CA" signer-ca-true "Signer CA True" "$PERSONNEL_OU" 3072 "$BAD_EXT" signer_ca_true
issue "$ROOT_CA" signer-rsa2048 "Signer RSA 2048" "$PERSONNEL_OU" 2048 "" v3_end_entity
issue "$ROOT_CA" holder-with-eku "Holder With EKU" "$PERSONNEL_OU" 3072 "$BAD_EXT" holder_with_eku
issue "$FOREIGN_CA" foreign-tsa "TSA Ajena de Prueba" "$TSA_OU" 3072 "$TSA_CNF" v3_tsa
fact cert server-signer.der partner root.der -
fact cert holder.der declaration root.der -
fact cert revoked-holder.der declaration root.der -
fact cert tsa.der time-stamping root.der -
fact cert tsa-noncritical-eku.der time-stamping root.der eku-not-critical
fact cert signer-ca-true.der partner root.der basic-constraints-ca-true
fact cert signer-rsa2048.der partner root.der rsa-2048-key
fact cert holder-with-eku.der declaration root.der eku-email-protection-present
fact cert foreign-tsa.der time-stamping foreign-root.der -

for name in server-signer holder; do
    quiet openssl x509 -in "$ROOT_CA/certs/$name.crt.pem" -pubkey -noout \
        -out "$WORK/$name.pub.pem"
done

# revoke NAME: revoke a root-issued certificate; every revocation is dated.
revoke() {
    PKI_CA_DIR="$ROOT_CA" quiet openssl ca -config "$OPENSSL_CNF" \
        -revoke "$ROOT_CA/certs/$1.crt.pem"
    awk -F '\t' -v OFS='\t' -v at="$REVOKED_AT_TEXT" '$1 == "R" { $3 = at } { print }' \
        "$ROOT_CA/index.txt" > "$WORK/index.txt"
    mv "$WORK/index.txt" "$ROOT_CA/index.txt"
}

# CRLs from number 1000 hex: empty, revoking the revoked holder, the same
# list ending before every genTime, and also revoking the time-stamping one.
gen_crl "$ROOT_CA" root-empty.crl
revoke revoked-holder
gen_crl "$ROOT_CA" root.crl
gen_crl "$ROOT_CA" root-stale.crl "${REVOKED_AT_TEXT/#/20}" "$THIS_UPDATE_TEXT"
revoke tsa
gen_crl "$ROOT_CA" root-tsa-revoked.crl
gen_crl "$FOREIGN_CA" foreign.crl
for name in root-empty root root-stale root-tsa-revoked; do
    fact crl "$name.crl.der" root.der
done
fact crl foreign.crl.der foreign-root.der

# PEM copies of the material packages carry as PEM entries.
for name in root foreign-root server-signer holder revoked-holder tsa foreign-tsa; do
    quiet openssl x509 -inform DER -in "$STAGE/$name.der" -out "$STAGE/$name.pem"
done
for name in root.crl root-empty.crl root-stale.crl root-tsa-revoked.crl foreign.crl; do
    quiet openssl crl -inform DER -in "$STAGE/$name.der" -out "$STAGE/$name.pem"
done

# Document and detached signatures over it.
printf 'evidence-verify fixture document\nfixed bytes for detached signatures\n' \
    > "$STAGE/document.bin"
DOCUMENT_DIGEST="$(sha256_hex "$STAGE/document.bin")"
printf 'evidence-verify fixture other covered object\n' > "$WORK/other.bin"
OTHER_DIGEST="$(sha256_hex "$WORK/other.bin")"
sign server-signer server-signer.sig document.bin
sign holder holder.sig document.bin

# Statements signed by the holder.
python3 -I "$HELPER" statements --root "$STAGE/root.der" \
    --holder "$STAGE/holder.der" --document "$STAGE/document.bin" \
    --holder-signature "$STAGE/holder.sig" \
    --binding-out "$STAGE/certificate-binding.bin" \
    --consent-out "$STAGE/document-consent.bin"
sign holder certificate-binding.sig certificate-binding.bin
sign holder document-consent.sig document-consent.bin
fact statement certificate-binding.bin certificate-binding.sig holder.der
fact statement document-consent.bin document-consent.sig holder.der

# Time-stamping authorities. The test configuration is pki/tsa.cnf plus
# sections that copy internal_tsa and fix the ESS hash or admit a second
# policy; every section keeps certs = the root chain, the policy, the
# accuracy and the ordering of the deployment.
ROOT_TSA="$WORK/root-tsa"
FOREIGN_TSA="$WORK/foreign-tsa"
new_tsa "$ROOT_TSA" "$ROOT_CA" tsa
new_tsa "$FOREIGN_TSA" "$FOREIGN_CA" foreign-tsa
TSA_BODY="$(sed -n '/^\[ internal_tsa \]/,/^\[/{/^\[/d;p}' "$TSA_CNF")"
grep -q '^default_policy *= *'"$POLICY" <<<"$TSA_BODY" \
    || die "internal_tsa in $TSA_CNF no longer uses policy $POLICY"
tsa_section() {
    printf '\n[ %s ]\n%s\n' "$1" "$TSA_BODY"
    shift
    printf '%s\n' "$@"
}
TSA_TEST_CNF="$WORK/tsa-test.cnf"
{
    cat "$TSA_CNF"
    tsa_section tsa_ess_v1 'ess_cert_id_alg = sha1'
    tsa_section tsa_ess_v2 'ess_cert_id_alg = sha256'
    tsa_section tsa_other_policy 'ess_cert_id_alg = sha256' \
        "other_policies = $OTHER_POLICY"
} > "$TSA_TEST_CNF"

stamp token-ess-v1.tsr tsa_ess_v1 "$ROOT_TSA" "$DOCUMENT_DIGEST" response
stamp token-ess-v2.tsr tsa_ess_v2 "$ROOT_TSA" "$DOCUMENT_DIGEST" response
stamp token-bare-v2.der tsa_ess_v2 "$ROOT_TSA" "$DOCUMENT_DIGEST" token
stamp token-other-policy.tsr tsa_other_policy "$ROOT_TSA" "$DOCUMENT_DIGEST" response \
    "$OTHER_POLICY"
stamp token-foreign.tsr tsa_ess_v2 "$FOREIGN_TSA" "$DOCUMENT_DIGEST" response
stamp token-other-digest.tsr tsa_ess_v2 "$ROOT_TSA" "$OTHER_DIGEST" response
fact token token-ess-v1.tsr response tsa.der root.der signing-certificate \
    "$DOCUMENT_DIGEST" document.bin "$POLICY"
fact token token-ess-v2.tsr response tsa.der root.der signing-certificate-v2 \
    "$DOCUMENT_DIGEST" document.bin "$POLICY"
fact token token-bare-v2.der token tsa.der root.der signing-certificate-v2 \
    "$DOCUMENT_DIGEST" document.bin "$POLICY"
fact token token-other-policy.tsr response tsa.der root.der signing-certificate-v2 \
    "$DOCUMENT_DIGEST" document.bin "$OTHER_POLICY"
fact token token-foreign.tsr response foreign-tsa.der foreign-root.der \
    signing-certificate-v2 "$DOCUMENT_DIGEST" document.bin "$POLICY"
fact token token-other-digest.tsr response tsa.der root.der signing-certificate-v2 \
    "$OTHER_DIGEST" - "$POLICY"

# Sanity checks with OpenSSL: every token verifies like the deployment
# verifier, the foreign and other-digest tokens fail where they must,
# chains verify, and the CRL revokes exactly the revoked holder.
ROOT_PEM="$STAGE/root.pem"
FOREIGN_PEM="$STAGE/foreign-root.pem"
for name in token-ess-v1.tsr token-ess-v2.tsr token-other-policy.tsr; do
    quiet check_token "$name" "$ROOT_PEM" "$DOCUMENT_DIGEST" response
done
quiet check_token token-bare-v2.der "$ROOT_PEM" "$DOCUMENT_DIGEST" token
quiet check_token token-foreign.tsr "$FOREIGN_PEM" "$DOCUMENT_DIGEST" response
quiet check_token token-other-digest.tsr "$ROOT_PEM" "$OTHER_DIGEST" response
must_fail check_token token-foreign.tsr "$ROOT_PEM" "$DOCUMENT_DIGEST" response
must_fail check_token token-other-digest.tsr "$ROOT_PEM" "$DOCUMENT_DIGEST" response
for name in server-signer holder tsa; do
    quiet openssl verify -x509_strict -crl_check -CAfile "$ROOT_PEM" \
        -CRLfile "$STAGE/root.crl.pem" "$STAGE/$name.pem"
done
quiet openssl verify -x509_strict -CAfile "$FOREIGN_PEM" "$STAGE/foreign-tsa.pem"
quiet openssl verify -x509_strict -CAfile "$ROOT_PEM" "$STAGE/revoked-holder.pem"
must_fail openssl verify -x509_strict -crl_check -CAfile "$ROOT_PEM" \
    -CRLfile "$STAGE/root.crl.pem" "$STAGE/revoked-holder.pem"
must_fail openssl verify -x509_strict -crl_check -CAfile "$ROOT_PEM" \
    -CRLfile "$STAGE/root-tsa-revoked.crl.pem" "$STAGE/tsa.pem"
for name in root.crl root-empty.crl root-stale.crl root-tsa-revoked.crl; do
    quiet openssl crl -in "$STAGE/$name.pem" -CAfile "$ROOT_PEM" -noout
done
quiet openssl crl -in "$STAGE/foreign.crl.pem" -CAfile "$FOREIGN_PEM" -noout

python3 -I "$HELPER" index --dir "$STAGE" --facts "$FACTS" \
    --generated-at "$NOW" --out "$STAGE/fixtures.json"

# Delete every private key and refuse to publish anything that looks
# like one.
rm -rf "$KEYS" "$ROOT_CA/private" "$FOREIGN_CA/private" \
    "$ROOT_TSA/private" "$FOREIGN_TSA/private"
if grep -rlq 'PRIVATE KEY' "$STAGE"; then
    die "private key material found in the staged fixtures"
fi

mkdir -p "$OUT_DIR"
find "$OUT_DIR" -mindepth 1 -maxdepth 1 ! -name README.md -exec rm -rf {} +
cp "$STAGE"/* "$OUT_DIR"/
printf 'Wrote %s files to %s\n' "$(find "$STAGE" -type f | wc -l)" "$OUT_DIR"
