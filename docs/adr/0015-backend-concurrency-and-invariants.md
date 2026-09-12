# ADR-0015: Backend concurrency and persisted invariants

## Status

Accepted for the local authenticated backend. Public deployment still requires
the operational work listed in `docs/backend-review.md`.

## Context

The backend combines synchronous application ports with Axum. Previously every
request could enqueue another blocking task, identity JSON accepted the same
16 MiB bound as documents, and separate route modules duplicated bearer parsing
and task execution. A cancelled HTTP future does not stop an already-running
blocking task. Argon2id uses 256 MiB per hash, so unbounded parallel requests
could exhaust memory even when application results were correct.

Review also exposed gaps between validation and persistence: MFA challenges
were looked up and removed in separate operations; failure counters could lose
their expiry; document replacement could overwrite another process's seal;
audit readers did not coordinate with writers; and deserialization bypassed
version and recovery-code invariants.

## Decision

Build the full server through `web::api_router`, with one shared runtime for
identity, document and case routes. Admit at most eight API requests before
body extraction and at most two blocking application operations by default.
Reject saturation immediately as `503 server_busy`, without an unbounded queue.
Expose positive limits through `--max-in-flight-requests` and
`--max-blocking-operations`. The liveness endpoint remains outside admission.

A blocking worker owns its semaphore permit until the closure finishes,
including when its HTTP future is cancelled. The request permit bounds body
extraction and handler execution; it is separate from the blocking-work permit.
There is no blanket timeout that would falsely imply a mutation was cancelled.
Response streaming and transport connections need separate deployment limits.
See the upstream [Tokio blocking-task documentation](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).

Identity and case JSON bodies have a 16 KiB maximum; document bodies retain
16 MiB. Identity request DTOs reject unknown fields. All authenticated routes
share bearer parsing: reject multiple Authorization values and whitespace in
tokens, and compare the authentication scheme without case sensitivity. API
responses include `Cache-Control: no-store`.

Split identity and document handlers into delivery modules and keep shared
transport behavior in `crates/web/src/request.rs` and
`crates/web/src/runtime.rs`. User creation receives a bearer token and verifies
the current account and permission in the application use case, rather than
trusting a caller-constructed Principal. Email validation rejects ASCII control
characters before normalization.

Consume each MFA challenge atomically before factor verification. Redis uses
GETDEL, so overlapping attempts cannot both issue sessions from one challenge.
A rejected attempt consumes that challenge, while replay claims and recovery
code optimistic revisions remain separate protections. Failure increments and
expiry updates run in one Redis Lua operation; existing windows are preserved.
Counter reads also restore a missing expiry using the application-provided
window, including legacy counters already above the login lockout threshold.
TCP connect and established-socket Redis I/O default to five seconds. The
current driver performs its handshake before those I/O settings can be applied;
this restriction is explicit and requires further work before deployment.

Use a per-document filesystem lock for insert and replace. Re-read the current
record under the lock, reject an already-sealed record, and preserve identity,
version, name, digest and encrypted vault on replacement. Atomic file rename
alone does not provide this conditional update guarantee. Audit readers hold
the same sibling lock in shared mode while writers hold it exclusively.

Both PostgreSQL adapters initialize schema through one helper, one transaction
and the same database-scoped advisory lock. This applies both migrations even
when the user adapter is constructed first.

Deserialize document versions through the nonzero constructor and use checked
increment. Deserialize recovery codes only when exactly eight slots exist,
including consumed slots. Preserve the existing valid JSON formats.

## Consequences

- Regression tests cover competing MFA attempts, stale account authorization,
  expiry recovery, cancelled HTTP futures, saturation, concurrent file updates,
  partial audit reads and mixed-adapter startup.
- A caller of `IdentityWorkflow::create_user` now supplies a bearer token;
  `SessionStore` consumes a challenge atomically. HTTP paths and successful
  JSON responses remain compatible; malformed input is rejected earlier.
- Worker limits bound application concurrency but do not provide fair per-user
  scheduling, distributed admission, TLS, database pooling or protection from
  slow network clients. Defaults must be validated on deployment hardware.
- Local locks protect cooperating processes on the same filesystem. They do
  not make document and audit writes transactional, anchor audit history
  externally, or associate documents with cases.
- Client document access stays denied. The next transactional and resource
  authorization work is specified in `docs/next-goal.md`.
