# Exact certificate bindings for an authenticated Owner

## Status

Accepted for structural domain values, canonical bytes and strict cryptographic
verification and application authorization. Audited persistence still needs its
own accepted implementation before a binding can be registered durably.

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
Future application callers must obtain these facts from the current authenticated
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
fields describe the intended registration capture. Future cryptographic admission
must reconstruct these bytes from validated inputs, hash them once with SHA-256
and verify the external signature under the registered Partner profile. An
untrusted client cannot provide a successful verification result.

Withdrawal uses expected/proposed binding revisions 1/2. It retains the original
deployment, root, trust revision, binding UUID and leaf fingerprint; its account
revision/generation describe the current Owner performing the withdrawal. The
original registration bytes remain intact. Withdrawal bytes identify the
authenticated decision for a future audit receipt; they are not a request to
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

These service checks and real verifier dispatch are covered locally. Repository
methods remain ports: their database transaction, serialization, unique live
binding and permanent fingerprint ownership require separate backend evidence.
The service does not expose an HTTP endpoint or enable certificate login.

### Persistence and admission obligations

The domain models a supplied registration and its terminal withdrawal without
claiming that a signature was verified, a row committed or a session issued.
The later application/repository boundary must establish all of the following:

- A current authenticated Owner, the same account, and exact expected account
  revision/generation are rechecked under the mutation locks.
- A dedicated strict Partner verifier enforces certificate purpose and mandatory
  published root/CRL trust. The declaration profile must keep its existing EKU
  rejection. Uploaded leaf material never selects a trust anchor.
- Cryptographic work occurs outside the audit lock; trust and the validity
  interval are checked again using time read after waiting for that lock.
- Only one binding is current for an account, and an exact certificate
  fingerprint cannot move between accounts. These are global persistence
  invariants, not promises made by a single in-memory domain value.
- Registration, public evidence and audit commit together. The same binding UUID
  resolves an uncertain registration by exact stored evidence; different bytes
  conflict. Withdrawal keeps the registration history and is committed with its
  audit event under the expected binding revision. A repeated domain transition
  rejects; repository receipt reconciliation does not repeat the mutation.
- Runtime cannot publish trust, change the authority or bypass existing member
  guards. Schema validation and restoration preserve the exact records and
  public evidence. No user counter changes are needed for the initial inert
  binding registry.

Certificate login is a later delivery. It needs a fresh single-attempt challenge,
explicit limits, the binding's origin through MFA and session admission, and
revocation/expiry checks against current published trust. Publishing a CRL does
not currently advance users.auth_generation. A binding registry alone must not
enable access or be reported as completed certificate authentication.

## Consequences

- Registration and withdrawal have deterministic, purpose-separated records
  with bounded values and terminal local transitions before SQL or HTTP design.
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
