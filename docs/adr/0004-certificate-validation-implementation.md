# 4. Certificate validation by re-encoding the parsed TBS structure

## Status

Accepted.

## Context

The prototype operates an internal certificate authority with the OpenSSL
scripts under `pki/`: an RSA-3072 root signs end-entity certificates and a
certificate revocation list (CRL), all with sha256WithRSAEncryption. The
Rust side must validate what those scripts produce: parse certificates,
check the signature chain, check the validity window, and check revocation.

The workspace pins the `x509-cert` 0.2 / `der` 0.7 crate family (see
`docs/adr/0001-msrv-and-dependency-pinning.md`). Those crates parse X.509
structures but deliberately do not verify signatures. The options for
signature checking were:

1. Shell out to `openssl verify` for every validation.
2. Use `ring` to verify PKCS#1 v1.5 signatures over the raw TBS bytes.
3. Re-encode the parsed to-be-signed (TBS) structure to DER with
   `der::Encode` and verify with the `rsa` and `sha2` crates against the
   issuer's subject public key info.

Option 1 makes every validation depend on a subprocess and makes the
outcome causes (expired versus revoked versus untrusted) fragile text
parsing. Option 2 requires carrying the exact original TBS byte span
through the parse, which the owned `x509-cert` types do not expose.

## Decision

Implement option 3 in `crates/infrastructure/src/certificates/`: re-encode
`TbsCertificate` (and `TbsCertList` for the CRL) with `der::Encode`, hash
with SHA-256, and verify the PKCS#1 v1.5 signature with the `rsa` crate
against the issuer's public key.

## Consequences

- Re-encoding is only correct because DER is canonical: for input that is
  valid DER, decode followed by encode reproduces the signed bytes.
  Certificates whose TBS encoding is not canonical DER fail signature
  verification and are reported as untrusted; this is acceptable for a
  single-layer internal authority whose only producer is OpenSSL, which
  emits canonical DER.
- Verification uses public keys only, so the accepted timing-channel
  advisory on the `rsa` crate's private-key operations
  (`docs/adr/0002-rsa-signing-crate-and-advisory.md`) does not weaken
  validation.
- Only sha256WithRSAEncryption is accepted. A certificate or CRL signed
  with any other algorithm cannot chain to the internal root and is
  reported as untrusted rather than causing an error.
- Interoperability with OpenSSL is pinned by tests that compare the
  validator's verdict with `openssl verify -crl_check` on the same
  fixtures (`crates/infrastructure/tests/certificate_validation.rs`).
