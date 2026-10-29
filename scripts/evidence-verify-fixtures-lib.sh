# evidence-verify-fixtures-lib.sh - helper functions of
# scripts/evidence-verify-fixtures.sh, which sources this file.
#
# The functions read the variables the generator sets before calling them:
# OPENSSL_CNF, NOT_BEFORE, NOT_AFTER, SUBJECT_BASE, KEYS, STAGE, WORK, FACTS,
# LOG, ROOT_CA, REVOKED_AT_TEXT, TSA_BODY and TSA_TEST_CNF. Sourcing it
# defines functions only and runs nothing.
# Rationale: docs/adr/0085-in-browser-evidence-validation.md.

die() {
    printf 'evidence-verify-fixtures.sh: error: %s\n' "$1" >&2
    exit 1
}

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

# revoke NAME: revoke a root-issued certificate; every revocation is dated.
revoke() {
    PKI_CA_DIR="$ROOT_CA" quiet openssl ca -config "$OPENSSL_CNF" \
        -revoke "$ROOT_CA/certs/$1.crt.pem"
    awk -F '\t' -v OFS='\t' -v at="$REVOKED_AT_TEXT" '$1 == "R" { $3 = at } { print }' \
        "$ROOT_CA/index.txt" > "$WORK/index.txt"
    mv "$WORK/index.txt" "$ROOT_CA/index.txt"
}

# tsa_section NAME [LINE...]: an authority section that copies the
# internal_tsa body of pki/tsa.cnf followed by the given lines.
tsa_section() {
    printf '\n[ %s ]\n%s\n' "$1" "$TSA_BODY"
    shift
    printf '%s\n' "$@"
}
