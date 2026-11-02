# evidence-verify-contract-variants.sh - variant ceremonies of
# scripts/evidence-verify-contract-fixtures.sh, which sources this file.
#
# Each variant starts from a copy of the base package entries, repeats the
# part of the ceremony it changes with the same keys and authority, and
# keeps only the entries that differ under variants/NAME, named as package
# entries so a test overlays them on the full or the partial package:
#
# - large-crl: parte-1-crl.pem with 2000 synthetic revoked serials, over
#   64 KiB, and the manifest and Tf over it (valid);
# - revoked-after-completion: crl-actual.der, a current CRL issued after Tf
#   that revokes the Client after Tf (valid; served, not a package entry);
# - other-request: a Client statement and signature over another request;
# - before-issue: request, Client statement and T1 of a request created a
#   day after T1, with its partial acknowledgement;
# - expired: a whole ceremony whose request expires between T1 and Tf;
# - same-key: a whole ceremony whose firm signer certificate holds the
#   Client key;
# - same-account: firm binding and statement naming the Client account;
# - foreign-t1: T1 over the Client signature from an authority under a
#   different root;
# - inverted-tokens: a Td requested before T1, with the manifest and Tf.
#
# The functions read the variables and functions of the generator and of
# scripts/evidence-verify-fixtures-lib.sh: BASE, BASE_STATE, WORK, KEYS,
# STAGE, ROOT_CA, TSA_DIR, TSA_CNF, OPENSSL_CNF, NOT_BEFORE, NOT_AFTER,
# SUBJECT_BASE, THIS_UPDATE, TF, helper, begin_ceremony, finish_ceremony,
# sign_entry, stamp_entry, gen_time, utc_time and general_time. Sourcing it
# defines functions and one entry list only and runs nothing.
# Rationale: docs/adr/0086-bilateral-contract-signing.md.

# start_variant NAME: point PACKAGE and STATE at a fresh copy of the base
# entries and state. Call it inside a subshell.
start_variant() {
    PACKAGE="$WORK/variant-$1"
    STATE="$WORK/variant-$1.json"
    mkdir -p "$PACKAGE"
    find "$BASE" -maxdepth 1 -type f -exec cp {} "$PACKAGE"/ \;
    cp "$BASE_STATE" "$STATE"
}

# keep_variant NAME ENTRY...: copy each ENTRY of the work copy into
# variants/NAME of the output; an entry equal to the base is an error.
keep_variant() {
    local target="$BASE/variants/$1" name
    shift
    mkdir -p "$target"
    for name in "$@"; do
        ! cmp -s "$PACKAGE/$name" "$BASE/$name" || die "variant entry equals the base: $name"
        cp "$PACKAGE/$name" "$target/$name"
    done
}

# large_crl: the first root list again, with the same window and 2000
# synthetic revoked serials dated a day before it, written to
# $WORK/large-crl.pem. The CA database is restored afterwards.
large_crl() {
    local index="$ROOT_CA/index.txt"
    cp "$index" "$WORK/index-before-large.txt"
    awk -v at="$(utc_time $(( THIS_UPDATE - 86400 )))" 'BEGIN {
        for (i = 1; i <= 2000; i++)
            printf "R\t360101000000Z\t%s\t7A%030X\tunknown\t/CN=Revocado Sintetico %d\n", at, i, i
    }' >> "$index"
    gen_crl "$ROOT_CA" large.crl
    mv "$WORK/index-before-large.txt" "$index"
    cp "$ROOT_CA/crl/large.crl.pem" "$WORK/large-crl.pem"
}

# early_document_token: a Td over the PDF requested before T1, written to
# $WORK/early-td.tsr; returns once the clock is past its genTime.
early_document_token() {
    local early
    STAGE="$WORK" stamp early-td.tsr internal_tsa "$TSA_DIR" \
        "$(sha256_hex "$PACKAGE/contrato.pdf")" response
    early="$(gen_time "$WORK/early-td.tsr")"
    while [ "$(date -u +%s)" -le "$early" ]; do sleep 1; done
}

# client_token: the Client signature and T1 over it in the work copy.
client_token() {
    sign_entry client parte-1-declaracion.bin
    stamp_entry parte-1-declaracion.sig.tsr parte-1-declaracion.sig
}

variant_large_crl() {(
    start_variant large-crl
    cp "$WORK/large-crl.pem" "$PACKAGE/parte-1-crl.pem"
    [ "$(wc -c < "$PACKAGE/parte-1-crl.pem")" -gt 65536 ] || die "the large CRL is not over 64 KiB"
    helper manifest
    stamp_entry contrato-manifiesto.json.tsr contrato-manifiesto.json
    keep_variant large-crl parte-1-crl.pem contrato-manifiesto.json contrato-manifiesto.json.tsr
)}

variant_inverted_tokens() {(
    start_variant inverted-tokens
    cp "$WORK/early-td.tsr" "$PACKAGE/contrato.pdf.tsr"
    [ "$(gen_time "$PACKAGE/contrato.pdf.tsr")" -lt "$(gen_time "$PACKAGE/parte-1-declaracion.sig.tsr")" ] \
        || die "the early Td is not before T1"
    helper manifest
    stamp_entry contrato-manifiesto.json.tsr contrato-manifiesto.json
    keep_variant inverted-tokens contrato.pdf.tsr contrato-manifiesto.json \
        contrato-manifiesto.json.tsr
)}

variant_other_request() {(
    start_variant other-request
    begin_ceremony --request-id 7a1c2a3b-4d5e-4f60-8a71-b2c3d4e5f6f2
    sign_entry client parte-1-declaracion.bin
    keep_variant other-request contrato-solicitud.bin parte-1-declaracion.bin \
        parte-1-declaracion.sig
)}

variant_before_issue() {(
    local now t1
    start_variant before-issue
    now="$(date -u +%s)"
    begin_ceremony --created-at $(( now + 86400 )) --expires-at $(( now + 8 * 86400 ))
    client_token
    t1="$(gen_time "$PACKAGE/parte-1-declaracion.sig.tsr")"
    helper records --client-accepted-at "$t1" --firm-accepted-at "$t1" \
        --completed-at "$t1" --exported-at $(( t1 + 1 ))
    keep_variant before-issue contrato-solicitud.bin parte-1-declaracion.bin \
        parte-1-declaracion.sig parte-1-declaracion.sig.tsr contrato-acuse-parcial.json
)}

# The entries a whole variant ceremony rewrites in the full package.
CEREMONY_ENTRIES=(contrato-solicitud.bin parte-1-declaracion.bin parte-1-declaracion.sig
    parte-1-declaracion.sig.tsr parte-2-declaracion.bin parte-2-declaracion.sig
    contrato.pdf.tsr contrato-manifiesto.json contrato-manifiesto.json.tsr
    contrato-recibo.json)

variant_expired() {(
    local expires t1 tf
    start_variant expired
    expires=$(( $(date -u +%s) + 6 ))
    begin_ceremony --expires-at "$expires"
    client_token
    t1="$(gen_time "$PACKAGE/parte-1-declaracion.sig.tsr")"
    [ $(( t1 + 1 )) -le "$expires" ] || die "the expired variant T1 is already past expires_at"
    while [ "$(date -u +%s)" -le "$expires" ]; do sleep 1; done
    finish_ceremony firm
    tf="$(gen_time "$PACKAGE/contrato-manifiesto.json.tsr")"
    [ $(( tf + 1 )) -gt "$expires" ] || die "the expired variant Tf is within expires_at"
    keep_variant expired "${CEREMONY_ENTRIES[@]}"
)}

variant_same_key() {(
    local csr="$ROOT_CA/csr/same-key.csr.pem" cert="$ROOT_CA/certs/same-key.crt.pem"
    start_variant same-key
    PKI_CA_DIR="$ROOT_CA" quiet openssl req -config "$OPENSSL_CNF" -new -sha256 \
        -key "$KEYS/client.key.pem" -out "$csr" \
        -subj "$SUBJECT_BASE/OU=Personal del Despacho/CN=Firmante con la Clave del Cliente"
    PKI_CA_DIR="$ROOT_CA" quiet openssl ca -config "$OPENSSL_CNF" -batch -notext \
        -md sha256 -startdate "$NOT_BEFORE" -enddate "$NOT_AFTER" \
        -extensions v3_internal_declaration -in "$csr" -out "$cert"
    quiet openssl x509 -in "$cert" -outform DER -out "$STAGE/same-key.der"
    quiet openssl x509 -inform DER -in "$STAGE/same-key.der" \
        -out "$PACKAGE/parte-2-certificado.pem"
    begin_ceremony --firm-der "$STAGE/same-key.der"
    sign_entry client parte-2-vinculo.bin
    client_token
    finish_ceremony client
    keep_variant same-key "${CEREMONY_ENTRIES[@]}" parte-2-certificado.pem \
        parte-2-vinculo.bin parte-2-vinculo.sig
)}

variant_same_account() {(
    start_variant same-account
    begin_ceremony --firm-as-client
    sign_entry firm parte-2-vinculo.bin
    helper counter
    sign_entry firm parte-2-declaracion.bin
    keep_variant same-account parte-2-vinculo.bin parte-2-vinculo.sig \
        parte-2-declaracion.bin parte-2-declaracion.sig
)}

variant_foreign_t1() {(
    local foreign="$WORK/foreign-ca" digest
    start_variant foreign-t1
    new_root "$foreign" 5000 "$SUBJECT_BASE/OU=Autoridad Ajena/CN=Raiz Ajena de Prueba"
    issue "$foreign" foreign-tsa "TSA Ajena de Prueba" "Autoridad de Sellado de Tiempo" \
        3072 "$TSA_CNF" v3_tsa
    new_tsa "$WORK/foreign-tsa" "$foreign" foreign-tsa
    digest="$(sha256_hex "$PACKAGE/parte-1-declaracion.sig")"
    STAGE="$PACKAGE" stamp parte-1-declaracion.sig.tsr internal_tsa "$WORK/foreign-tsa" \
        "$digest" response
    quiet openssl ts -verify -digest "$digest" -in "$PACKAGE/parte-1-declaracion.sig.tsr" \
        -CAfile "$foreign/ca.crt.pem"
    must_fail openssl ts -verify -digest "$digest" \
        -in "$PACKAGE/parte-1-declaracion.sig.tsr" -CAfile "$PACKAGE/ca.pem"
    keep_variant foreign-t1 parte-1-declaracion.sig.tsr
)}

# variant_revoked_after_completion: once the clock is past the Tf interval,
# re-date the Client revocation to now and issue a current list from now.
variant_revoked_after_completion() {
    local now target="$BASE/variants/revoked-after-completion"
    while [ "$(date -u +%s)" -le $(( TF + 1 )) ]; do sleep 1; done
    now="$(date -u +%s)"
    awk -F '\t' -v OFS='\t' -v at="$(utc_time "$now")" '$1 == "R" { $3 = at } { print }' \
        "$ROOT_CA/index.txt" > "$WORK/index.txt"
    mv "$WORK/index.txt" "$ROOT_CA/index.txt"
    gen_crl "$ROOT_CA" after-completion.crl "$(general_time "$now")" \
        "$(general_time $(( now + 7 * 86400 )))"
    must_fail openssl verify -x509_strict -crl_check -CAfile "$BASE/ca.pem" \
        -CRLfile "$ROOT_CA/crl/after-completion.crl.pem" "$BASE/parte-1-certificado.pem"
    mkdir -p "$target"
    cp "$STAGE/after-completion.crl.der" "$target/crl-actual.der"
}

generate_variants() {
    variant_large_crl
    variant_inverted_tokens
    variant_other_request
    variant_before_issue
    variant_expired
    variant_same_key
    variant_same_account
    variant_foreign_t1
    variant_revoked_after_completion
}
