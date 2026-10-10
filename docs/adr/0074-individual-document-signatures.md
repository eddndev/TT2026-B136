# Individual document signatures with externally held keys

## Status

Accepted for implementation. The complete workflow remains unaccepted until its
certificate issuance, account binding, document signing, persistence, HTTP,
Qadra, restoration and verification criteria pass together.

## Context

Document sealing currently uses one configured server signer. An authenticated
actor initiating that operation is not the individual cryptographic signer.
The internal CA issues certificates, but its existing issuance command also
generates subject keys inside its working directory. The individual workflow
must keep those keys outside the CA and application server.

Existing encrypted document versions, signature/timestamp evidence and exports
must remain readable without reinterpretation. The account-certificate login
workflow has a different purpose and does not authorize document signing.

## Decision

Add administrative issuance from a bounded externally signed RSA-3072 CSR, using
the existing CA and controlled signing-only certificate extensions. Verify CSR
proof, key parameters and the exact issued public key. Do not copy requested
extensions or create a subject private key. Existing CA commands remain intact;
this delivery adds no HTTP CA, certificate authority hierarchy or external PSC.

A currently authorized Litigator binds their own public certificate through a
separately signed canonical registration containing the account, deployment,
operation and published trust references. Account identity does not follow from
a subject name or email. Registration, current lookup and terminal withdrawal
preserve exact receipts and audit history. A fingerprint cannot be reassigned
to another account by withdrawing its previous binding.

For an exact PDF version, keep the detached signature of its content digest so
existing independent content verification remains available. Require an
additional signature of an explicit canonical consent that binds the content
signature to the account, operation, binding, case, document, version, content
digest and published trust revision. These two signatures are produced outside
the server; the UI receives public certificates and signatures only. Different
purposes have distinct canonical prefixes. Another account, document version,
operation or signature cannot reuse that consent.

The existing timestamp service seals the content digest and its response is
verified before commit. This proves the timestamped content digest under the
configured TSA trust; it does not establish that the personal signature already
existed at the token's generation time. A timeout or invalid token leaves no
completed signature. No fallback TSA is selected implicitly.

Persist individual public evidence as an explicit optional, versioned extension
of sealed evidence, omitted for historical server seals. Preserve an exact
operation receipt, binding provenance and captured trust. Under the shared audit
transaction, revalidate the complete principal, account counters, case access,
active case, unwithdrawn binding, unchanged document and current trust. Read the
clock after lock waits. Evidence and its receipt/audit commit atomically. Exact
replay returns the original receipt after current authorization; a different
intent conflicts. No automatic browser retry is introduced.

Verification and exports preserve the content signature, signed consent,
certificate, registration proof, captured trust and timestamp. They distinguish
historical validity from current certificate/binding/trust status. Historical
server seals are labelled as such, never attributed retroactively to a litigant.
Qadra reuses its document/version selection and in-memory draft recovery and
requires explicit confirmation of the PDF and all signing references.

## Consequences

A complete personal signature requires both cryptographic proofs, the exact
registered certificate and an authorized atomic commit. Preparing a registration
or emitting a certificate alone does not complete this delivery. External key
custody means signing is performed on the holder's machine; the application must
provide exact files and reproducible instructions without accepting a private
key, PKCS#12 file, key password or authentication secret.

Acceptance covers issuance, binding/withdrawal, valid signature, wrong purpose
or content, stale/revoked credentials, lost authorization, timestamp failure,
concurrent replay, audit rollback, migration/restoration, export verification and
the real desktop/mobile flow. Existing valid evidence is reused. The completed
functional delivery receives one closing regression and exact-revision gates.
No new counters, legal profiles, invitations, CA portal or certificate-login
changes belong to this scope.
