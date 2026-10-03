# Owner certificate binding HTTP contract

The standalone `web::owner_certificate_router` exposes registration evidence for
the currently authenticated Owner. It does not enable certificate login or
personal document signing. The binary composition is a separate delivery; a
standalone router test does not establish that these routes are deployed.

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

## Errors and work limits

Responses use `Cache-Control: no-store`. Missing/invalid session returns 401 and
current insufficient permission returns 403. Other stable error groups are
invalid input (400), absent own binding (404), conflicting account/trust/binding
state (409), rejected credential (422), oversized input (413), exhausted
admission (503) and internal failure (500). Conflict and credential groups use
neutral messages rather than publishing internal verification details.

The standalone factory installs its HTTP admission protection once. Blocking
application work uses its bounded runtime; the bearer remains inside a
zeroizing allocation captured by the worker. Composition into an existing API
must share that API's runtime and admission instead of adding another budget.

See [the binding decision](adr/0067-owner-certificate-bindings.md) for canonical
evidence, PostgreSQL transactions, trust and the separate certificate-login
boundary. Local HTTP acceptance uses the real application service with controlled
ports; it does not substitute for real RSA/PostgreSQL transport acceptance.
