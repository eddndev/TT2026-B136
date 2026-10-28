# 0085 - In-browser evidence validation and the sealed evidence manifest

## Status

Accepted. This record fixes the validation contract and the evidence manifest
format before their implementation. It extends
[ADR 0079](0079-browser-local-cryptography.md), whose delivery excluded a ZIP
verifier, and keeps the server verification strategy of
[ADR 0005](0005-rfc3161-verification-strategy.md) unchanged.

## Context

### Exported packages

Qadra exports two evidence packages as stored ZIP files written by
`crates/infrastructure/src/archive.rs`. The writer uses method 0, flags 0,
version needed 10, version made by 20 (MS-DOS), external attributes 0, the
DOS date 1980-01-01 at time 0, no extra fields, comments, data descriptors or
ZIP64, and contiguous entries from offset 0 followed by one central directory
and one end record. Entry names follow `crates/domain/src/crypto/archive.rs`:
1 to 128 bytes of ASCII letters, digits, `.`, `-` and `_`, starting with a
letter or digit, compared byte for byte.

The server seal package (`crates/application/src/evidence.rs`) contains the
document `{name}` (any admitted format), its detached signature `{name}.sig`,
its RFC 3161 response `{name}.tsr`, `certificado.pem` (the configured seal
signer), `ca.pem` (that signer's configured issuer), `crl.pem` (the configured
CRL file read when `serve` starts), `tsa-chain.pem` and `INSTRUCCIONES.md`.
Current `serve` always writes `tsa-chain.pem`; records sealed without a chain,
including some imported legacy records and command line packages, export
without it, and the token anchor is then `ca.pem`.

The individual signing package (`crates/application/src/evidence/individual.rs`)
contains the same names with a different meaning, plus `document-consent.bin`,
`document-consent.sig`, `certificate-binding.bin`, `certificate-binding.sig`,
`binding-root.der`, `binding-crl.der` and `individual-signing.json`. The
document is a PDF. `{name}.sig` and `certificado.pem` belong to the holder.
`ca.pem` is the holder's credential trust root and must decode to the same DER
as `binding-root.der`. `crl.pem` is that root's CRL at the acceptance trust
revision; it equals `binding-crl.der` when registration and acceptance used the
same revision. `tsa-chain.pem` holds the root that anchors the time-stamping
certificate carried inside the token; `pki/issue-tsa-cert.sh` copies the CA
root into it, so in the documented deployment it has the same bytes as
`ca.pem`. `individual-signing.json` holds the binding and consent statements
and signatures, the registration trust and the instants
`binding_registered_at` and `accepted_at` as integer seconds; byte fields are
JSON arrays of integers.

The PDF time-stamp covers the document digest only. It does not show that the
detached signature or the consent existed at the stamped time. The final
`accepted_at` is assigned inside the database transaction, after both
signatures have been verified and the PDF token has been obtained.

### Certificates and time-stamps

`crates/infrastructure/src/certificates/internal_profile/profile.rs` and
`revocation.rs` define strict profiles. The server applies them to Owner
credentials, individual signing and CA inspection. Certificates issued by
`pki/issue-cert.sh` with `v3_end_entity` have the shape of its `Partner`
profile and are used as the server seal signer, but the seal path checks that
signer with `crates/infrastructure/src/certificates/validator.rs`, which does
not inspect key usage, extended key usage or key identifiers. Time-stamp
tokens are verified by `openssl ts -verify -digest ... -CAfile` at the current
time, without a CRL, with OpenSSL's own purpose checks.

The internal authority is configured by `pki/tsa.cnf`: SHA-256 imprints and
signer digest, policy `1.3.6.1.4.1.13762.3`, accuracy of one second, no
ordering, a nonce in every request, and the CA root appended to the token
certificates. Its signer info uses `rsaEncryption` with SHA-256. The
configuration does not set `ess_cert_id_alg`: tokens exported by the current
deployment carry the ESS `SigningCertificate` attribute with a SHA-1
certificate hash, while tokens produced with the same configuration by a newer
OpenSSL carry `SigningCertificateV2` with SHA-256.

### Browser building blocks

`cms` 0.2, `x509-tsp` 0.1 and `x509-cert` 0.2 decode the CMS, RFC 3161 and
X.509 structures; they offer signing builders but no signature, chain or
time-stamp verification, and none of them defines the ESS signing certificate
types. Decoding `signedAttrs` into their types sorts and re-encodes the set,
losing the signed bytes. These crates, `rsa` 0.9, `sha1` 0.10 and `sha2` 0.10
build for `wasm32-unknown-unknown` at the versions locked in `Cargo.lock`; the
`domain` crate does not, because `uuid` with the `v4` feature needs a random
source on that target.

## Decision

### A pure verification crate shared by browser and server

Add a workspace crate `crates/evidence-verify` with no dependency on any other
workspace crate and no I/O, clock, random source, process or database access.
It admits the stored ZIP, identifies the package profile, verifies RSA
signatures, certificates, CRLs, the canonical binding and consent statements,
CMS SignedData time-stamp tokens and the evidence manifest, and returns a
structured report. A separate `cdylib` wrapper compiled to WebAssembly exposes
it to a dedicated Web Worker. It is not added to `browser-crypto`, so public
verification stays apart from the module that opens private keys. Server
crates may depend on `evidence-verify`, and the server builds the manifest with
its canonical encoder; `evidence-verify` never depends on server crates.

The crate reimplements, from the code cited above, the byte layouts of
`crates/domain/src/document_signing.rs` and `consent.rs` and the certificate
and CRL rules of `internal_profile`. Native parity tests compare it with the
`domain` codec and with `internal_profile` on shared fixtures. Parity with
`openssl ts -verify` is required for the CMS signature, ESS, imprint and chain
checks over DER fixtures; the admitted policy, pinning, strict profiles and DER
strictness, which OpenSSL does not apply in the server call, have their own
fixtures with fixed expected results.

### Admission

Before any cryptographic check, the archive must have exactly the writer's
shape described in the context: stored entries only, contiguous from offset 0,
local and central headers that agree, one end record without comment, no data
descriptors, extra fields, encryption, ZIP64 or non-zero external attributes,
and names in the safe set. A package that does not is not supported. A CRC-32
mismatch is a failed package. Limits, on uncompressed entry sizes: at most 32
entries; 16 MiB for the document; 1 MiB for each CRL entry; 4.5 MiB for
`individual-signing.json`, whose integer arrays expand the 1 MiB binding CRL;
64 KiB for each other entry; 25 MiB in total. Larger packages are not
supported. ASN.1 nesting is bounded. `INSTRUCCIONES.md` is auxiliary,
optional and never interpreted.

### Package profiles

After admission the inventory, not entry order, selects one profile:

| Profile | Required entries | Optional | Covered objects |
| --- | --- | --- | --- |
| Server seal v1 | `{name}`, `.sig`, `.tsr`, `certificado.pem`, `ca.pem`, `crl.pem` | `tsa-chain.pem` | Document digest signed by the server seal signer and time-stamped |
| Individual signing v1 | The above with `tsa-chain.pem`, the four statement entries, `binding-root.der`, `binding-crl.der`, `individual-signing.json` | none | Document digest signed by the holder and time-stamped; binding and consent signed by the holder |
| Individual signing v2 | Individual signing v1 plus `manifiesto.json`, `manifiesto.json.tsr`, `tsa-crl.pem`, `recibo-aceptacion.json` | none | Individual signing v1 plus every manifest artifact, time-stamped through the manifest |

The document entry is the single name, other than the fixed names of the
candidate profile, that has both a `.sig` and a `.tsr` sibling. Fixed names
are evaluated per profile, so a server seal document may be named like an
individual entry. Unknown entries, a missing required entry or an ambiguous
document make the package not supported.

### Strict certificate and CRL profiles per role

Every package, including historical server seals, is checked with the strict
profile of each role. The normative definition is the code in
`internal_profile/profile.rs` and `revocation.rs`, summarized here:

| Rule | Root | Server seal signer | Individual signer | Time-stamping |
| --- | --- | --- | --- | --- |
| Shape | `Root` | `Partner` | `Declaration` | `TimeStamping` |
| Basic constraints (critical) | CA, no path length | not CA | not CA | not CA |
| Key usage (critical) | certificate and CRL signing | digital signature, non-repudiation | digital signature, non-repudiation | digital signature |
| Extended key usage | absent | non-critical, exactly client authentication and e-mail protection | absent | critical, exactly time-stamping |
| Other extensions | subject key identifier only | subject and authority key identifiers | subject and authority key identifiers | subject and authority key identifiers |

All roles require X.509 v3, `sha256WithRSAEncryption` with NULL parameters and
a 384-byte signature, RSA-3072 with exponent 65537, a non-zero serial of at
most 20 bytes, no unique identifiers, the subject key identifier equal to the
SHA-1 of the public key, and for non-root certificates a key-identifier-only
authority key identifier of 20 bytes equal to the issuer's subject key
identifier. The root must be self-signed. CRLs must be v2 with the same
signature algorithm, exactly a non-critical authority key identifier and a
non-critical CRL number of at most 8 bytes, a `nextUpdate` after `thisUpdate`,
at most 10,000 entries with unique non-zero serials of at most 20 bytes and no
entry extensions, revocation dates not after `thisUpdate`, and the issuer name
of the root.

The report keeps three findings apart: the cryptographic signature over the
exact bytes (correct or incorrect); certificate and trust (the strict profile
and the chain to an admitted root are met or not); and missing evidence
(indeterminate). A correct signature with a failed profile is not reported as
an altered document. Packages are never rewritten.

### Time-stamp tokens

A `.tsr` entry is a `TimeStampResp` whose status is granted (0) or
grantedWithMods (1) and which carries a token, or a bare token; any other
status or a missing token is failed. The token must be a ContentInfo with
SignedData version 3 whose encapsulated content type is `id-ct-TSTInfo` with
content present, and exactly one SignerInfo. Its identifier is
issuerAndSerialNumber with version 1 or subjectKeyIdentifier with version 3
and must match exactly one certificate in the token; certificates inside the
token are used for path building only, never as anchors. The digest algorithm
is `id-sha256` with absent or NULL parameters and is listed in
`digestAlgorithms`; the signature algorithm is `rsaEncryption` or
`sha256WithRSAEncryption` with absent or NULL parameters; anything else is not
supported.

The crate locates the exact `signedAttrs` and encapsulated content bytes with
its own bounded DER walker. The signature is verified over those bytes with
the first identifier octet changed from `[0]` to SET OF, never over a
re-encoding; non-DER input is malformed. The signed attributes must include
`contentType` equal to `id-ct-TSTInfo` and `messageDigest` equal to SHA-256 of
the encapsulated content. At least one ESS attribute must be present, and each
present one must match the signer certificate: `SigningCertificate` with the
SHA-1 hash of its DER, or `SigningCertificateV2` with an `ESSCertIDv2` whose
absent hash algorithm means SHA-256; a present `issuerSerial` must match. The
ESS types are defined in the crate. `TSTInfo` must be version 1 with no
critical extension; its imprint must be SHA-256 of the covered object;
`signingTime`, `tsa`, the nonce and ordering are reported but not relied on.

A token whose policy is outside the admitted list, initially
`1.3.6.1.4.1.13762.3`, or whose certificate does not chain to a root pinned
for time-stamping, is not supported. A token that does chain but fails a CMS,
ESS, imprint or profile check is failed.

### Revocation outcomes

For every certificate role the same rules apply at that role's instant: a CRL
of the right issuer that does not cover the instant is indeterminate; a CRL of
another issuer, with an invalid signature, malformed, or failing the CRL
profile is failed; a listed serial revoked at or before the instant is
failed, even when that CRL does not cover the instant, because the profile
admits no certificate hold; a later revocation leaves the historical result
checked and is shown in the current status. The time-stamping certificate's revocation needs a CRL
from its issuer: `crl.pem` when the issuers match, and in v2 `tsa-crl.pem`,
which decides for the manifest token. When both apply, a revocation in either
fails the check. Byte-identical CRL entries are both kept and both covered.

### Instants and consistency

Each check is evaluated at its own instant:

- the time-stamping certificate and its CRL at each token's `genTime`;
- the server seal signer at the document token's `genTime`;
- the binding statement at `binding_registered_at`, with `binding-root.der` and
  `binding-crl.der`;
- the holder's document signature and consent at `accepted_at`, with `ca.pem`
  and `crl.pem`.

A validity window must contain the whole interval `genTime` plus or minus the
token accuracy. `binding_registered_at` and `accepted_at` are application
records: in v1 nothing covers them, and the report says so. The validator
requires `binding_registered_at <= accepted_at`, document token `genTime` not
after `accepted_at` beyond the accuracy, and in v2 document `genTime` not after
the manifest `genTime` and the manifest `genTime` not after `accepted_at`
beyond the accuracy, and applies the registration and acceptance consistency
rules of `crates/application/src/documents/individual_evidence.rs` (same
certificate and root, equal trust for equal revisions, otherwise a larger CRL
number and a non-decreasing `thisUpdate`). A violation is failed.

### Component results and the overall result

Each component has one result: checked, failed, indeterminate for lack of
evidence, not supported, or not applicable.

| Component | Server seal v1 | Individual v1 | Individual v2 |
| --- | --- | --- | --- |
| Package | mandatory | mandatory | mandatory |
| Document digest and token | mandatory | mandatory | mandatory |
| Document signature | mandatory | mandatory | mandatory |
| Binding and consent | not applicable | mandatory | mandatory |
| Certificates and trust | mandatory | mandatory | mandatory |
| Revocation, including the TSA | mandatory | mandatory | mandatory |
| Manifest and its token | not applicable | not applicable | mandatory |
| Context (instants, records, receipt) | mandatory | mandatory | mandatory |
| Current status | informational | informational | informational |

The overall result takes the first that applies: failed if any mandatory
component failed; not supported, with its reason, if any mandatory component is
not supported or no profile matches; partial if any mandatory component is
indeterminate; otherwise complete. Informational results never change it. The
context component checks the instants and consistency rules above and, in v2,
the receipt. The current status reports validity windows and revocation at the
caller's instant, from the trust route below, and names the client clock as
the source of that instant.

Historical server seals whose captured `crl.pem` does not cover their token
`genTime`, because the server read it when it started, are partial.

### Trust anchors

A `ca.pem`, `tsa-chain.pem`, `binding-root.der` or token certificate is never
trusted because it is present. A root is admitted only when its SHA-256
fingerprint is listed for the needed purpose in the pinned set built into the
Qadra release: `web/apps/qadra/src/lib/evidence/pinned-roots.json`, a list of
`{fingerprint, purposes, label}` with purposes `server-seal`,
`individual-signing` and `time-stamping`. One root may serve several purposes,
and roots that once anchored packages stay listed. Builds for other
environments replace the file as a build input.

The read-only route `GET /api/v1/evidence/trust`, on the main router for any
principal accepted by `IdentityWorkflow::authenticate`, returns what the server
currently admits, reusing the field names of the Owner certificate trust
responses: the seal root and CRL from the configured evidence material, the
time-stamping anchor from `tsa-chain.pem`, and, when individual signing is
configured, the current credential trust snapshot with `deployment_id`, its
revision and CRL. It is not under a case because local files have no case. The
validator contrasts it with the pinned set: a root pinned but absent from the
route stays checked and the difference is reported; a root returned by the
route but not pinned is indeterminate trust; an unavailable route changes only
the current status. No URL found inside an artifact is followed.

### The evidence manifest

New individual signatures add `manifiesto.json`, a time-stamp over its digest
and the time-stamping issuer CRL. The manifest is UTF-8 JSON in the canonical
form of RFC 8785, restricted to objects, arrays, strings and non-negative
integers up to 2^53 - 1; floats, `null`, booleans, duplicate keys and unknown
fields are rejected. The validator re-serializes the parsed value and fails the
manifest unless the result equals the entry bytes, with no byte order mark or
trailing newline. Digests are lowercase hexadecimal SHA-256 and the case,
document, operation, deployment, account and binding identifiers are lowercase
hyphenated UUIDs.

Top-level keys: `profile` (`qadra-evidence-manifest`), `profile_version` (1),
`purpose` (`individual-document-signing`), `deployment_id`, `case_id`,
`document_id`, `document_version`, `document_sha256`, `operation_id`,
`algorithms` (`digest`: `sha-256`, `signature`: `rsa-pkcs1v15-sha256`,
`timestamp`: `rfc3161-sha256`), `artifacts` and `signers`.

`artifacts` lists every covered entry once, sorted by entry name, as
`{entry, role, sha256, size}`. Roles are unique: `document`,
`document-signature`, `document-timestamp`, `signer-certificate`,
`issuer-certificate`, `issuer-crl`, `tsa-chain`, `tsa-issuer-crl`,
`binding-statement`, `binding-signature`, `binding-root`, `binding-crl`,
`consent-statement` and `consent-signature`.

`signers` holds `{id, account_id, binding_id, binding_registered_at, entries}`
per holder, where `id` is `signer-1`, `signer-2` and so on, and `entries` maps
exactly `document_signature`, `signer_certificate`, `binding_statement`,
`binding_signature`, `binding_root`, `binding_crl`, `consent_statement` and
`consent_signature` to entry names listed in `artifacts` with the matching
role. One holder's document and consent signatures form one signer. Each
signer's consent statement must carry the manifest's operation, case,
document, version and `document_sha256`, the digest of its document signature,
and identity fields matching the signer. This profile admits exactly one
signer; a manifest with more is not supported until a later profile defines
multi-party completion and consent.

The manifest does not contain its own digest, its token,
`individual-signing.json` (its `accepted_at` is final only after commit), the
instructions or the receipt. `recibo-aceptacion.json`, generated at export from
stored data, is RFC 8785 JSON with exactly `profile`
(`qadra-acceptance-receipt`), `profile_version` (1), `operation_id`,
`manifest_sha256`, `accepted_at_seconds`, `accepted_at_nanoseconds` and
`provenance` (`application-record`). No time-stamp covers it. A receipt whose
seconds differ from `individual-signing.json`, or whose digest differs from the
computed manifest digest, fails the context component.

### Generation inside the acceptance operation

`DocumentSigningPorts` gains a time-stamp service, its verifier, the anchor and
a source of the time-stamping issuer CRL. The CRL source is the current
credential trust snapshot when its root issued the time-stamping certificate;
otherwise v2 acceptance is rejected as a configuration error. Before requesting
any token, acceptance rejects a document name `S` for which `S`, `S.sig` or
`S.tsr` equals, ignoring ASCII case, a fixed name of any profile; upload
applies the same reservation.

After the holder's signatures, binding and consent are verified and the PDF
token is obtained, the service builds the manifest from the final bytes,
requests a token over its SHA-256 and verifies it against the same anchor.
A manifest token whose `genTime` precedes the document token's is rejected,
because the authority does not guarantee ordering. It checks the captured CRL: issued by the token certificate's issuer, signature
valid under the anchor, current at that moment, not listing the time-stamping
certificate, at most 1 MiB. It then repeats the existing time check and
reauthentication. Authority calls stay outside the SQL transaction. If any
earlier step fails, no manifest token is requested; if the manifest token or
the CRL check fails, nothing is committed. The service does not retry.
Resubmitting the same operation is the retry and obtains new tokens; once an
operation is accepted, its stored manifest and token are returned unchanged.

The manifest, its token and the CRL are stored in a sibling table keyed by the
operation, inserted in the acceptance transaction, protected by immutability
triggers, described by its own schema inventory module, and checked by a
deferred constraint trigger: every receipt created after the migration has
exactly one manifest row whose operation, document, version and digests match.
The startup inventory recomputes each stored manifest's digests. The trigger
count expected for `individual_document_signatures` changes with the new
foreign key. `individual-signing.json` keeps `format_version` 1. Export loads
the sibling row with the record and adds `manifiesto.json`,
`manifiesto.json.tsr`, `tsa-crl.pem` and `recibo-aceptacion.json` before
`INSTRUCCIONES.md`; repeated exports return identical bytes.

### Interface

Qadra offers the validator next to the existing evidence download of the
selected document version and accepts a local ZIP. Local files are processed in
the worker and never uploaded. The worker runs one validation, reports stages,
can be cancelled, and its late results are discarded by operation identifier
and document context. The report can be downloaded as JSON with the engine
version, profile, policy, pinned and route anchors, instants and their sources,
digests, and every component result with its reason; it is a description of
the run, not a new signature or seal.

### Outside this decision

Time-stamping authorities other than the internal one, a global entry point
outside the document view, replacing OpenSSL in the server verifier, a server
signature over the manifest and multi-party signing are not part of it.

## Consequences

People can check evidence from Qadra without a terminal, with results that
state which object each time-stamp covers and which policy was applied.

The verifier is new security-relevant code. Its fixtures cover: altered
document or signature; crossed consent; an unknown root inside the ZIP; wrong
certificate purpose; a missing, future, expired, foreign or revoking CRL; a
token over another digest; truncated DER; contradictory JSON; an ambiguous or
oversized package; an altered or swapped manifest; a token of another
manifest; and an omitted referenced artifact.

Historical packages keep their bytes. Their strict verdict can differ from the
server's, which checks the seal signer loosely and the time-stamping
certificate at the current time without a CRL; seals made long after the
server last started are partial. The interface explains these differences.

Acceptance of an individual signature makes a second time-stamp request,
lengthening the window in which the commit may find expired material, and a
clock step at the authority can reject an acceptance that resubmission then
completes.

Individual signing requires the credential trust root to be the issuer of the
time-stamping certificate. A deployment with separate roots cannot accept
individual signatures until a separate time-stamping CRL source is defined. The
documented deployment meets the requirement: its `tsa-chain.pem` and `ca.pem`
are the same root.

The migration adds foreign key triggers that earlier binaries count at startup,
so they refuse to start once it is applied. Returning to an earlier version
requires restoring the pre-deployment backup and, as described in
[database operations](../database-operations.md), reconciling the acceptances
and other writes made after it.
