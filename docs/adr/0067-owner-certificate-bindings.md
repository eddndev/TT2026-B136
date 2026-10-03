# Exact certificate bindings for an authenticated Owner

## Status

Accepted for structural domain values, canonical bytes, strict cryptographic
verification, application authorization and audited PostgreSQL persistence.
Certificate login, HTTP exposure and complete restore acceptance remain separate.

## Context

The identity workflow uses password authentication followed by MFA. A certificate
uploaded to the application is public material; it establishes neither private
key possession nor an association with an account. A subject name, email address
or represented participant identity cannot supply that association.

The internal authority already issues Partner certificates with clientAuth and
emailProtection usages. Participant declarations use a different signing profile
and bind represented identities independently of access accounts, as described
in [ADR-0026](0026-internal-participant-declarations.md). Their statements cannot
be reused as account bindings, login proofs or document signatures.

Before certificate login can be admitted, the application needs an exact,
auditable association with an existing account and a terminal withdrawal. This
association must not silently change password/MFA behavior, account privileges,
document signing or the existing server signing credential.

## Decision

### Scope and authority

Allow only an active Owner to bind a certificate to that same Owner account.
Application callers obtain these facts from the current authenticated
principal and durable account, and revalidate them when committing. A pure
domain constructor checks the supplied facts; it does not authenticate them.

Use the existing UserId, UUID and Sha256Digest representations. Account,
deployment and binding UUIDs must be non-nil; no UUID version is required.
Account revision and authentication generation are integers in 0..=i64::MAX,
with generation no greater than revision, matching the member inventory.
Trust revision is a positive u32. Digests are opaque 32-byte values: this layer
does not parse certificates, recompute fingerprints or invent forbidden hashes.

The supplied binding UUID identifies one immutable registration. It is neither
derived from the certificate fingerprint nor generated inside the domain.
Registration creates binding revision 1 with an absent expectation, encoded as
zero. Withdrawal requires revision 1 and produces terminal revision 2. There is
no edit, replacement, reactivation or additional revision of the same binding.
A later registration requires another binding UUID and a new explicit signature.

Withdrawal requires the same active Owner and cannot use an account revision or
generation older than the registration capture. Equal account counters are
valid because this operation does not mutate users. Withdrawal does not require
a new certificate signature or current certificate/CRL validity: the owner must
be able to remove a lost or expired credential using the existing MFA session.

### Canonical record

Both records contain exactly 150 bytes. UUIDs retain their 16-byte representation;
integers use unsigned big-endian encoding; fingerprints use 32 raw bytes. No JSON,
text UUID, variable-length string, account email, subject name or private material
enters the canonical record.

| Offset | Length | Value |
| --- | ---: | --- |
| 0 | 8 | ASCII `OWNCERT1` |
| 8 | 1 | Purpose: registration 1, withdrawal 2 |
| 9 | 1 | Policy: internal Partner account credential 1 |
| 10 | 16 | Deployment UUID |
| 26 | 32 | Root certificate DER SHA-256 |
| 58 | 4 | Captured trust revision |
| 62 | 16 | Owner UserId |
| 78 | 8 | Account revision |
| 86 | 8 | Authentication generation |
| 94 | 16 | Binding UUID |
| 110 | 4 | Expected binding revision |
| 114 | 4 | Proposed binding revision |
| 118 | 32 | Leaf certificate DER SHA-256 |

Registration uses expected/proposed binding revisions 0/1. Its account and trust
fields describe the intended registration capture. Cryptographic admission
reconstructs these bytes from validated inputs, hashes them once with SHA-256
and verifies the external signature under the registered Partner profile. An
untrusted client cannot provide a successful verification result.

Withdrawal uses expected/proposed binding revisions 1/2. It retains the original
deployment, root, trust revision, binding UUID and leaf fingerprint; its account
revision/generation describe the current Owner performing the withdrawal. The
original registration bytes remain intact. Withdrawal bytes identify the
authenticated decision for its audit receipt; they are not a request to
sign with the lost key and do not claim a fresh trust inspection.

The prefix, purpose and policy form one fixed domain separator. Constructors
cannot select arbitrary values for them. These records are not login challenges,
participant declarations or document-signing payloads. Byte identity is tested
against literal independent hexadecimal vectors, including nontrivial integer
byte order. Cryptographic verification and anti-replay persistence are separate
obligations; changing canonical bytes alone is not evidence of their enforcement.

### Strict public-material verification

`InternalRsaOwnerBindingVerifier` accepts a typed registration statement, public
leaf certificate, detached signature, supplied trust snapshot and explicit time.
It computes SHA-256 over the 150 canonical bytes internally. It cannot accept
an arbitrary digest or change the statement purpose.

The Partner profile requires RSA-3072 with exponent 65537, exact SHA-256/RSA
algorithm parameters, critical digitalSignature/nonRepudiation key usage and
exactly the two noncritical clientAuth/emailProtection extended usages. The
participant-declaration profile continues to reject any extended key usage.
Private profile selection shares bounded DER/PEM parsing, issuer and CRL checks
without broadening either profile. A signed current CRL remains mandatory.

The verifier recomputes the complete trust inspection and compares it with the
supplied snapshot, including derived fingerprints and validity bounds. The
statement must match deployment, trust revision, root and leaf. Successful
verification proves consistency with supplied material at the supplied time;
it does not prove that a public Rust snapshot came from the current published
database state. The caller must load that state and recheck it at commit.
No private key is received, generated or persisted by this verifier.

### Application boundary

`OwnerCertificateService` authenticates the current MFA session through the
existing identity port. Preparation loads the account and published trust through
the repository port, then constructs an opaque registration containing the exact
canonical statement and inspected public certificate. Registration verifies the
signature outside persistence locks, reads time after verification and compares
the full authenticated principal again before committing the opaque verified
command. Session tokens are borrowed for the call and are not retained in it.

Receipt and withdrawal operations require the current active Owner and exact
binding UUID. An expired certificate or superseded trust does not prevent reading
an original receipt or withdrawing that credential. Exact uncertain-registration
reconciliation compares original statement, certificate, signature and whole
trust before returning history. A concurrent terminal withdrawal preserves the
first withdrawal's original counters and time; it cannot replace another
registration or produce another revision. Errors do not trigger implicit retry.

The repository port is implemented by `PostgresOwnerCertificateStore`, with
separate local evidence for transaction atomicity, concurrent receipts, unique
live bindings and permanent fingerprint ownership. The service does not expose
an HTTP endpoint or enable certificate login.

### Untrusted public submission

A public submission carries exactly the 150 prepared statement bytes, canonical
leaf DER and detached 384-byte signature. It cannot deserialize an opaque
preparation or assert successful verification. The application checks an own
receipt by exact binding UUID first and compares all public bytes, preserving
original evidence after expiry, trust rotation or terminal withdrawal.

Without a receipt, it reconstructs preparation from the current account and
published trust and requires byte identity before verification. A changed
capture conflicts rather than replacing the signed intention. The original full
Principal remains stable across nested service calls and before return. A failed
or uncertain commit is not retried. This bridge does not expose HTTP, enable
certificate login or change document signing.

### Audited PostgreSQL boundary

The three `0029_owner_certificate_*.sql` migrations persist registrations and
terminal withdrawals in separate append-only tables. The adapter accepts the
application's opaque commands and begins an audited READ COMMITTED transaction.
It obtains the shared audit transaction lock and locks the user row before
rechecking that the actor is the same active Owner. A fresh mutation requires
the exact prepared account counters and principal. Fresh registration also
compares the complete currently published trust with the prepared snapshot.

Mutation-time RSA verification occurs before these persistence locks. The
adapter reads time after all authority waits and rechecks the admitted interval
before inserting. A later registration cannot precede its account's previous
terminal withdrawal. SQL guards repeat structural, account, trust and audit
checks; they do not perform RSA or authenticate an MFA session.

Only one unwithdrawn binding may exist per account. A leaf fingerprint cannot
move to another account, even after withdrawal. The shared audit lock serializes
both adapter commits and direct INSERT guards, including checks for absent
UUIDs, fingerprints and live bindings. Immutable evidence rows need no UPDATE
row lock or UPDATE grant; the current user row still receives its own lock.

Each mutation appends its exact audit event and evidence in the same transaction.
The two audit foreign keys are nondeferrable. Actor email, action, binding and
Owner UUIDs, statement digest, seconds and nanoseconds must match the linked
event. A rejected audit insertion rolls back the mutation. No user field or
authentication generation changes when registering or withdrawing a binding.

Exact UUID reconciliation requires current Owner authority, then compares the
original public evidence before checking mutable counters, current trust or
expiry. It returns the first receipt without writing another event or reviving
a withdrawn binding. Different evidence conflicts. Concurrent withdrawal keeps
the first terminal statement, counters and time; it requires neither current
trust nor a still-valid certificate.

Runtime receives SELECT and explicit INSERT column grants on these tables, with
no UPDATE, DELETE, TRUNCATE or direct trigger-function EXECUTE. INVOKER guards
use a fixed `pg_catalog` search path. Catalog validation checks exact columns,
constraints, indexes, functions, triggers and audit attachments. Privilege checks
include roles reachable through MEMBER, even with NOINHERIT, and reject authority
to SET `session_replication_role` directly or through SET ROLE. Such authority
could bypass ordinary guards and foreign-key triggers. Existing trust publication
and member restrictions remain in force.

Migration and runtime admission validate historical inventory. Structural checks
reconstruct canonical bytes, digests, validity intervals, history ordering and
exact audit links. Current account counters cannot precede any captured
registration or withdrawal counter; a later role or activity change does not
invalidate historical evidence. The public certificate and signature are then
reverified using the captured trust and `checked_at`, not today's trust or clock.
The recomputed cryptographic inspection must equal the stored inspection.

This inventory establishes the new rows' exact audit associations; it does not
replace the existing global audit-chain verifier. Backend tests separately use
that verifier. They exercise real PostgreSQL and RSA with a controlled identity
port, not a complete password/MFA login. Inventory rejection of inconsistent
state is not evidence of a completed dump/restore campaign; that operational
acceptance and broader adversarial catalog coverage remain separate.

Certificate login is a later delivery. It needs a fresh single-attempt challenge,
explicit limits, the binding's origin through MFA and session admission, and
revocation/expiry checks against current published trust. Publishing a CRL does
not currently advance users.auth_generation. A binding registry alone must not
enable access or be reported as completed certificate authentication.

## Consequences

- Registration and withdrawal have deterministic, purpose-separated records
  with bounded values, immutable audited storage and terminal withdrawals.
- Domain tests can prove exact bytes, account authority checks over supplied
  facts, counter bounds and preservation of registration on withdrawal. They do
  not prove MFA, RSA, trust publication, concurrency or persistence.
- Real-user identity checking, issuance, custody, delivery, loss and renewal of
  private keys remain outside this delivery. The current CA script generates a
  private key at the issuer; a synthetic holder fixture does not prove exclusive
  personal custody. No new private-key upload path is introduced.
- The existing internal CA is not FIREL, e.firma or a public certification
  provider. Document signing still needs its own explicit consent, exact content
  authorization and personal signature; neither a binding nor a login proof
  supplies that document authorization.
- Current password/MFA login, roles, sessions and server sealing stay unchanged.
  No public activation setting or operational certificate policy is introduced.
