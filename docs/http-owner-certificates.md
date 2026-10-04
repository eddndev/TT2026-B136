# Owner certificate binding HTTP contract

The full API and standalone `web::owner_certificate_router` expose registration
evidence for the currently authenticated Owner. The `serve` binary composes the
real application service with its existing identity and validated PostgreSQL
store. These operations do not enable certificate login or personal document
signing. The four UUID-based operations have the real HTTP/MFA/RSA/PostgreSQL and
SQL-restore acceptance described below; the later `/current` query has separate
focused acceptance. This extension is not yet published, installed or exposed
through a Qadra interface.

## Routes and input

All paths start with `/api/v1/auth/certificate-bindings`. A `{binding_id}` is a
non-nil UUID selected once by the caller; it need not be UUID v4. The literal
`/current` route discovers the caller's own unwithdrawn binding without that UUID.
Every operation requires the current MFA-authenticated bearer session and current
Owner authority for the account. Query strings are rejected.

| Method and suffix | JSON object or body | Result |
| --- | --- | --- |
| GET `/current` | Empty body | 200 with the complete own unwithdrawn receipt or JSON `null`, without a wrapper |
| POST `/{binding_id}/prepare` | `certificate_base64` | Canonical statement and current public certificate/trust capture; no mutation |
| POST `/{binding_id}/register` | `statement_base64`, `certificate_der_base64`, `signature_base64` | Original committed receipt or a new registration |
| GET `/{binding_id}` | Empty body | Own historical receipt; absent binding returns 404 |
| POST `/{binding_id}/withdraw` | `expected_revision` as an unsigned 32-bit integer | Terminal withdrawal receipt |

JSON requests use `application/json` and a maximum body of 32 KiB. Objects reject
unknown or duplicate keys, positional arrays and trailing JSON. Binary values
use canonical standard padded Base64. The declaration is exactly 150 decoded
bytes, the signature exactly 384 bytes, and certificate input is 1 to 16,384
bytes. Registration carries canonical leaf DER. No private key is accepted.

Preparation returns `binding_id`, `owner_id`, `policy`, `statement_base64`,
`account_revision`, `auth_generation`, `certificate`, `deployment_id`,
`trust_revision` and `root_fingerprint`. The caller signs the exact decoded
statement; it cannot declare its own successful cryptographic verification.
The server rebuilds a fresh preparation from current state and requires exact
byte identity before admitting new evidence.

## Receipts and reconciliation

`current` means no terminal withdrawal at the serialized read. It does not mean
that the certificate or trust is currently valid, nor reserve that state after
the response. The query rechecks active Owner authority under the existing
audit/account locks and rejects multiple unwithdrawn rows. It appends no audit
event and requires no new RSA verification or current trust publication.
The service reauthenticates the full principal before returning either evidence
or absence. A later withdrawal makes the next query return `null`; the old UUID
route keeps its exact terminal history. Queries and nonempty bodies are rejected.

A receipt contains binding/owner identifiers, binding revision and policy, the
original `registration`, and a nullable terminal `withdrawal`. Registration
includes the statement, statement digest, public certificate and detached
signature, captured account counters, checked/validity instants, registration
time and complete public historical trust. Withdrawal includes its exact
statement, captured account counters and original withdrawal time.

Account revisions, authentication generations and CRL numbers are decimal
strings so JavaScript does not round values above its safe-integer range.
Audit-associated timestamps retain nanosecond precision in RFC 3339 text.
Certificate output includes canonical DER, fingerprint and public summary.
Trust includes deployment/revision, root and CRL DER, fingerprints/digests,
CRL number and validity, and publication time/actor.

After a lost registration response, consult the exact binding UUID or resubmit
exactly the same public bytes. A matching historical receipt is considered
before current trust or certificate expiry. Different evidence conflicts;
withdrawal is terminal and cannot revive the registration. Current Owner
session/authority is still required for every reconciliation. The adapter does
not automatically retry an uncertain commit.
Withdrawal reauthenticates the original full principal after an applied or
concurrent-existing commit before releasing evidence; lost authority rejects the
response without undoing or retrying a confirmed mutation.

## Errors and work limits

Responses use `Cache-Control: no-store`. Missing/invalid session returns 401 and
current insufficient permission returns 403. Other stable error groups are
invalid input (400), absent own binding at an exact UUID (404), conflicting account/trust/binding
state (409), rejected credential (422), oversized input (413), exhausted
admission (503) and internal failure (500). Conflict and credential groups use
neutral messages rather than publishing internal verification details.

The full API merges these routes into its existing `HttpRuntime` before the
single admission layer. They share request and blocking-work limits with the
other routes. Blocking capacity also covers consumers using the injected
`HttpWorkBudget`. Exhausted
HTTP admission returns the same `503 server_busy` before reading a request body.
Authentication through the application service also waits for a blocking-work
permit. Cancelling the HTTP request does not release a permit held by a running
synchronous operation; that worker retains it until completion.

The standalone factory creates its own bounded runtime and installs protection
once. Full composition uses the private route group, not this standalone factory.
The 32 KiB body limit applies only to Owner binding input. The bearer remains
inside a zeroizing allocation captured by the worker.

## Server composition and evidence limits

`CaseWorkflows.owner_certificates` supplies the required
`Arc<OwnerCertificateService>`. All full-API constructors use it, including the
constructor accepting an external work budget. In `serve`, the service uses the
existing identity instance, `PostgresOwnerCertificateStore`, the strict Partner
verifier and SHA-256 implementation. Application and store share a clock instance.
The store opens inside `with_validated_postgres` without migrations or grant
repair. No new activation flag, issuer selection or private-key input is added.

New registrations still require current published trust. Historical receipt and
withdrawal keep their existing authority requirements and do not require a
currently valid certificate. These routes do not change MFA or session admission.

See [the binding decision](adr/0067-owner-certificate-bindings.md) for canonical
evidence, PostgreSQL transactions, trust and the separate certificate-login
boundary. The [verification report](verification-report.md) records standalone
and shared-runtime tests with controlled ports separately from backend and
integrated real-service acceptance.

## Real-service acceptance

On 3 October 2026, `bash scripts/api-demo.sh` completed with exit 0 in 334.419 s,
including setup, against disposable PostgreSQL 16.15, Redis, real password/MFA
identity and the internal CA. The Owner extension reuses that campaign's server,
trust publication and SQL restore. It independently reconstructs the 150-byte
statement, signs outside HTTP and verifies the returned public proof with OpenSSL.

The campaign checked one audited registration, exact replay, altered-signature
rejection and byte conflicts. Publishing a successor CRL made an old preparation
conflict and a new proof from the revoked certificate fail; the historical receipt
remained exact and terminal withdrawal required no fresh signature. SQL rows,
digests, audit sequences and timestamps matched the receipt. All four routes
rejected the revoked bearer and a current Paralegal without mutation.

After SQL restoration and fresh MFA, the terminal receipt, original public proof
and audit evidence stayed identical. Replaying registration or withdrawal added
no event and did not revive the binding. This is not certificate login, private
key custody validation, a browser acceptance or operational SQL/RDB/PKI recovery.

The later `/current` addition passed application and HTTP tests with controlled
ports, plus two PostgreSQL tests for own-account isolation, withdrawal/renewal,
expired trust, mutation-free reads and authority loss during an audit-lock wait.
It was not included in the 334.419 s integrated campaign above. The
[verification report](verification-report.md) keeps those focal results separate.
