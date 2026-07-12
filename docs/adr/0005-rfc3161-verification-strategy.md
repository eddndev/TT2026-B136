# 0005. RFC 3161 verification strategy

## Status

Accepted

## Context

Trusted timestamps follow RFC 3161: an authority returns a response
whose token is a CMS SignedData structure encapsulating a TSTInfo
assertion, which binds a message imprint (digest algorithm and value)
to a generation time under the authority's signature and certificate
chain.

Verifying a token therefore means three different checks: the token
decodes, its imprint matches the digest being attested, and its CMS
signature and certificate chain hold up against a trust anchor.

The workspace parses DER with the der 0.7 crate family (x509-cert 0.2,
cms 0.2, x509-tsp 0.1), pinned for the declared minimum toolchain (see
docs/adr/0001-msrv-and-dependency-pinning.md). The cms 0.2 crate
decodes SignedData but offers no API to verify its signatures, and
x509-tsp 0.1 only defines the RFC 3161 structures. Implementing CMS
signature verification by hand (signed attributes, content-type and
message-digest attribute checks, chain building) is a substantial
cryptographic surface this prototype should not own. The openssl
command line tool implements exactly this check as `openssl ts
-verify`, and the repository already depends on openssl subprocesses
for its certificate authority scripts under `pki/`.

## Decision

Verification runs in two stages inside `Rfc3161Verifier`
(crates/infrastructure/src/timestamp/verify.rs):

1. Native stage: decode the input with x509-tsp and cms, accepting a
   full TimeStampResp or a bare token; extract TSTInfo; compare the
   imprint's algorithm and digest against the expected digest; read the
   generation time. Undecodable input is reported as a malformed token
   and an imprint difference as an imprint mismatch, both without
   spawning any subprocess.
2. Delegated stage: run `openssl ts -verify -digest <hex> -in <token>
   -CAfile <anchor>` as a subprocess for the CMS signature and
   certificate chain check. A refusal is reported as an untrusted token
   carrying openssl's diagnostic.

## Consequences

- Verification needs the openssl tool at runtime, a dependency the
  repository already carries for the authority tooling under `pki/`.
- Imprint mismatches and malformed tokens are diagnosed natively with
  precise causes, independent of openssl's error reporting.
- The token travels to the subprocess through scratch files, which the
  verifier removes after each check.
- Revisit this split when the parsing crates grow a signature
  verification API; the native stage already owns all the decoding
  needed to replace the subprocess.
