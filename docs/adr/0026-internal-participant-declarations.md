# Internal certificates and exact participant declarations

## Status

Accepted.

## Context

An uploaded certificate is public material. It does not establish control of
its private key, the civil identity of a directory entry, professional status
or judicial appointment. A certificate reference typed into a form provides
even less evidence. Participant records need an attributable distinction between
declared information, staff decisions and an actual cryptographic check.

The general certificate and document verifiers retain their historical contracts.
The internal authority's default leaves carry authentication and email EKUs.
Those purposes do not authorize an arbitrary declaration: when EKU is present,
the intended use must be compatible with it and with key usage.
[RFC 5280, extended key usage](https://www.rfc-editor.org/rfc/rfc5280.html#section-4.2.1.12).

## Decision

### Explicit internal trust profile

Use `internal_demo_v1`, a deliberately bounded root-to-leaf profile. It is not
a general PKIX validator or evidence of interoperability with an official FIREL.
The administrator publishes the root and CRL; an uploaded leaf cannot select
its own trust anchor. No network fetch, AIA, OCSP or intermediate discovery
participates in the check.

The root and leaf use X.509 v3, RSA-3072 with exponent 65537, rsaEncryption SPKI
and matching inner/outer SHA256-with-RSA algorithms with NULL parameters. The
root is self-signed, has critical CA=true and exactly keyCertSign/cRLSign. The
leaf has critical CA=false without path length, exactly digitalSignature and
contentCommitment in critical key usage, and no EKU. SKI and AKI must be coherent.
Duplicate extensions and extensions outside this profile are rejected, including
noncritical extensions whose meaning the adapter would otherwise ignore.

`pki/issue-declaration-cert.sh` selects this additional leaf profile. The CLI
selects it through `pki issue --purpose participant-declaration`. Existing
certificates and the default `partner` issuance remain interpretable under their
original contract. Issuance for declarations verifies the resulting leaf's
profile before reporting success, including when the operator supplies scripts.

Inputs contain one public certificate or CRL, as strict DER or a single matching
PEM block with only exterior ASCII whitespace. Certificates and roots are each
bounded to 16 KiB before parsing. A CRL is bounded to 1 MiB and 10,000 entries;
it must be a direct, complete v2 CRL with a trusted signature, coherent AKI,
u64 CRL number, unique positive serials of at most 20 bytes, and no entry
extensions. Delta, indirect and partitioned CRLs are outside the profile.
The CRL interval requires thisUpdate <= evaluation <= nextUpdate and a strictly
later nextUpdate; revocation dates cannot exceed thisUpdate. Missing, future,
expired or untrusted revocation information fails closed.

The detached signature is exactly 384 bytes, using RSASSA-PKCS1-v1_5 with
SHA-256. The application hashes the canonical declaration once; the verifier
checks the SHA-256 DigestInfo representation, without hashing the digest again.
[RFC 8017, signature operations](https://www.rfc-editor.org/rfc/rfc8017.html#section-8.2).

### Canonical statement and interpretation

`PCRED1` has exactly 218 bytes, in this order. UUIDs use their 16-byte form,
integers are unsigned big-endian, and digests contain 32 raw bytes.

| Field | Bytes |
| --- | ---: |
| ASCII `PCRED1` | 6 |
| Purpose 0 and policy 0 | 2 |
| Deployment UUID | 16 |
| Root certificate DER fingerprint | 32 |
| Case UUID and represented-subject UUID | 32 |
| Subject operation: Keep 0 or Append 1 | 1 |
| Expected and proposed subject revisions | 8 |
| Proposed or retained subject values digest | 32 |
| Participant UUID | 16 |
| Expected and proposed participant revisions | 8 |
| Typed role tag | 1 |
| Proposed participant values digest | 32 |
| Leaf certificate DER fingerprint | 32 |

Keep refers to one positive subject revision. Append advances 0 to 1 or a
positive revision to its immediate successor. Participant expectations likewise
advance 0 to 1 or to the next revision. Zero is an expectation for creation,
never an invented historical snapshot. Subject and participant values exclude
this credential's own signature and evidence to avoid a circular digest.

The signature attests to these exact values and their associations. It does not
prove the truth of a CURP, professional registration, name or judicial role.
Staff identity decisions remain separate audit evidence. The document server's
shared signing key and local timestamp authority cannot substitute for this
individual signature. The recorded check time is not a claimed signing time.

Authenticated preparation supplies stable proposed UUIDs, exact expectations,
values, digests, certificate fingerprint, statement bytes and a readable receipt.
It does not reserve or persist an incomplete subject or participant. The holder
signs outside the server and submits the public certificate and detached signature.
Private keys, key passwords and PKCS#12 containers are not accepted inputs.

### Published revocation and atomic admission

An immutable deployment authority fixes the root. Append-only trust revisions
record public DER material, digests, CRL number, admissible interval, publication
time and the administrative PostgreSQL `SESSION_USER`. This provenance is not a
fabricated product-user UUID or email. Publication uses an administrative
connection, an expected revision and the common audited READ COMMITTED boundary.
The runtime role has read access to trust material, without publication or
mutation privileges. Root rotation needs a separate explicit policy.

New CRLs must have a strictly increasing number and nondecreasing thisUpdate.
Publication captures the clock after waiting for the audit lock and checks the
interval again. A CA revocation becomes visible to this workflow when its new
CRL is generated and published, not when a local index file changes.

Participant preparation captures the published trust revision. Cryptographic
verification runs outside the audit lock. The final transaction independently
revalidates actor, case access, active status, subject/participant expectations,
support snapshots and the identical trust head. It reads its clock after the
lock wait and confirms that it remains inside the verified certificate/root/CRL
intersection. A competing trust publication or expired interval prevents the
mutation. No RSA work needs to run while holding the database audit lock.

The resulting revision, credential evidence and audit event commit together.
Evidence includes the exact statement, signature, certificate, trust revision,
digests, original subject/participant references and captured check result.
Failure rolls back the complete change. Historical inspection uses its captured
trust and time; today's CRL does not rewrite a previous result.

### Repetition, status and uncertain outcomes

UUID uniqueness and expected subject/participant revisions prevent applying the
same successful declaration twice. There is no Redis challenge: this signature
authorizes one exact change rather than an interactive login. A pending statement
may remain admissible while its expectations and cryptographic intervals allow
it. The system does not claim interactive freshness or an original signing time.

After an uncertain response, retain the submitted bundle and inspect the exact
proposed participant revision, even if a later revision already exists. Compare
statement digest, signature, certificate and bound references. Similar current
text is not evidence that this submission committed. Never create fresh UUIDs
or resubmit automatically to resolve that uncertainty.

An edit requiring a credential needs a new signature for its new expectations.
Status-only archive/reactivation preserves the origin reference of the previous
credential without requesting another personal signature or claiming that it
attests to the new organizational state. An updated subject does not replace
the exact subject revision captured by an older participant revision.

## Consequences

- The prototype can reproduce issuance, signing, admission and revocation with
  synthetic holders and an internal authority while preserving the distinction
  from official credentials and legally established identity.
- Owner and assigned Litigator manage participant records; they and assigned
  Paralegal read the authorized detail and evidence. Client remains denied.
  Compact lists, error messages and textual audit resources exclude personal
  identifiers and credential material. Existing document permissions continue
  to govern exact encrypted supports.
- Backup and restore must preserve deployment identity, all trust revisions,
  statement bytes, public materials, captured decisions and audit history.
  Restoring an entire older coherent database is not prevented by revision
  checks; external anchoring remains outside their guarantee.
- Public trust publication adds an operational duty: generate and publish CRLs
  after revocation and before the current CRL expires. Admission stops when no
  acceptable published revocation state is available.
