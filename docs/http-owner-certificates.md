# Owner certificate binding HTTP contract

The full API and standalone `web::owner_certificate_router` expose registration
evidence for the currently authenticated Owner. The `serve` binary composes the
real application service with its existing identity and validated PostgreSQL
store. These operations do not enable certificate login or personal document
signing. Router acceptance does not establish an installed deployment or a real
HTTP/RSA/PostgreSQL workflow.

## Routes and input

All paths start with `/api/v1/auth/certificate-bindings/{binding_id}`. The binding
identifier is a non-nil UUID selected once by the caller; it need not be UUID v4.
Every operation requires the current MFA-authenticated bearer session and current
Owner authority for the account. Query strings are rejected.

| Method and suffix | JSON object or body | Result |
| --- | --- | --- |
| POST `/prepare` | `certificate_base64` | Canonical statement and current public certificate/trust capture; no mutation |
| POST `/register` | `statement_base64`, `certificate_der_base64`, `signature_base64` | Original committed receipt or a new registration |
| GET (no suffix) | Empty body | Own historical receipt; absent binding returns 404 |
| POST `/withdraw` | `expected_revision` as an unsigned 32-bit integer | Terminal withdrawal receipt |

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
invalid input (400), absent own binding (404), conflicting account/trust/binding
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
and shared-runtime HTTP tests separately from the real PostgreSQL/RSA backend
campaign. The HTTP tests use the real application service with controlled ports;
they do not substitute for an integrated real-service transport acceptance.
