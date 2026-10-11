# ADR 0077: Durable access outcomes and versioned audit context

## Status

Accepted and implemented. The [finite matrix](../access-audit-matrix.md) maps
existing operations to transport and permission evidence, with native storage,
query, restoration and timing results in the
[verification report](../verification-report.md). This decision defines behavior;
acceptance receipts and exact-revision integration gates establish its execution
status. It does not attest a deployed proxy policy or production latency.

## Context

The existing audit chain records the exact timestamp, textual actor, action and
resource. Business mutations preserve their audit event in the same transaction.
The Owner query already provides bounded chronological selection and immutable
historical values, as described in [ADR 0061](0061-bounded-owner-audit-query.md).
Its historical actors are not uniformly account UUIDs and it has no reliable
historical network origin. Reconstructing them from current accounts would
invent provenance and change the meaning of prior records.

Authorization belongs to existing application workflows and transactional
stores. A rejected access can return before a business transaction begins or
after that transaction rolls back. Existing activity events therefore cannot
serve as a complete record of both admitted and rejected HTTP operations.
Operational MFA diagnostics have their own purpose and retention boundary;
they do not replace a durable event in the audit chain.

The HTTP runtime already bounds admitted requests and blocking work. Dropping
an HTTP future does not stop a synchronous worker. Any access recorder attached
only to the caller's final response would lose the outcome of work that survives
the caller's disconnection. The new record must also distinguish durable append
latency from the duration of password hashing, document processing or an entire
HTTP operation.

## Decision

### Preserve history and extend only recorded new context

Add an optional typed `AuditContext` to each audit event:

- `user_id`: observed account UUID or null;
- `ip_address`: observed IPv4/IPv6 address or null;
- `source`: `http`, `cli` or `internal`;
- `request_id`: server-assigned correlation UUID or null.

An absent context contributes no bytes to the historical event encoding. A
present context appends the `AUDCTX01` version marker, explicit presence tags,
network-order UUID/address bytes and an explicit source tag after the complete
legacy encoding. This extension occupies 12 to 60 bytes. Absence, nil UUIDs and
unspecified addresses remain distinct. The chain hash covers the extension.
The exact layout lives in
[`AuditContext`](../../crates/domain/src/audit/context.rs); existing event bytes
remain defined by [`AuditEvent`](../../crates/domain/src/audit/event.rs).

[Migration 0048](../../migrations/0048_audit_event_context.sql) adds nullable
`context_json`, bounded to 1024 bytes. Old rows keep null; there is no identity
or IP backfill. New appends use typed context and include it in hashing.
Readers, exact historical references, file snapshots and restore/import paths
must preserve both representations. Existing append-only privileges remain.
A populated migration/restoration is required evidence, not implied by the DDL.

Request context is propagated from an async request scope into the synchronous
scope around `HttpRuntime::run`. Successful session/account admission binds the
current UUID. Rebinding one request to another UUID fails; nested scopes and
unwinding restore the previous scope. The context supplements existing current
principal revalidation and never grants permission or revives a session.

The textual actor remains the captured value of a business event. A new access
event uses its observed UUID as actor, or `unavailable`. A supplied email or UUID
in a public login, invitation or MFA request is not itself an authenticated
principal. Preserve unavailable fields instead of inferring identity. CLI and
internal consumers retain an explicit origin without a fabricated HTTP IP,
request identifier or current user. A worker's captured requester remains
distinct from an interactive session that is no longer executing.

### Record one outcome after an admitted handler

The request admission owner retains the request permit while the handler and
durable append finish, including when the client disconnects. The synchronous
worker retains its own work permit. At most the admitted request population
can wait for the access writer; this does not create an unbounded logging queue.
Orderly shutdown must also drain admitted owners that outlive their connections
before dropping the runtime and audit adapters. Cancellation and shutdown are
separate acceptance scenarios; neither implies survival of abrupt process loss.

The access writer appends one `access.*` event to the shared chain after the
handler produces a response, and before that response is released. Existing
business events keep their original transactional boundary; they are not
replaced or duplicated as activity. The access outcome is selected from the
handler's observed HTTP status:

| Status | Action | Meaning |
| --- | --- | --- |
| 2xx | `access.granted` | The handler returned a successful response. |
| 401 or 403 | `access.denied` | Authentication or permission was rejected. |
| Other non-5xx responses | `access.rejected` | The operation did not yield a success response; includes ambiguous 404 and other 4xx. |
| 5xx | `access.failed` | The handler reported an operational failure. |

An accepted public request does not prove account existence, completed MFA,
mail delivery or a business mutation. A 404 does not distinguish a missing
resource from a concealed unauthorized resource. A 409 or 422 does not by
itself establish permission denial. These limits remain visible in the contract.

A request rejected with 503 before acquiring an admission permit has not
executed an admitted operation. It is outside this durable access population;
the existing operational diagnostics, including MFA capacity reporting, remain
separate. Health checks, unmatched arbitrary paths and static assets are outside
the finite product catalog. Registered HTTP method rejection is a transport
outcome, not a new business operation.

Store only the method, recognized route pattern and safely parsed resource
references. Do not retain query strings, headers, bodies, tokens, passwords,
codes, certificate material, arbitrary unknown paths or adapter error text.
The correlation identity is generated by the server; client-supplied request
identifiers never override it. Operational MFA reporting remains independent
from whether an access append is durably confirmed.

The writer returns success only after commit. If the writer fails, suppress the
handler response and return the existing generic 500. A business transaction
may already have committed before this failure. The 500 is not rollback proof,
does not restore a revoked session, and does not authorize an automatic retry.
Existing operation receipts and explicit reconciliation retain their meaning.
Do not recursively attempt to audit the audit failure or report a durable event
that the store did not confirm.

### Trust only an explicitly configured local proxy

Direct mode records the socket peer and ignores forwarded headers. Trusted
loopback-proxy mode is opt-in server configuration. It may use
`X-Forwarded-For` only when the socket peer is loopback and there is exactly one
header containing one valid scalar IP address. Lists, duplicate headers,
malformed values and missing values fall back to the socket peer; no value is
reconstructed from another header. A non-loopback peer always keeps its own IP.
If the adapter has no socket metadata, the origin remains unavailable.

The deployment's [Nginx template](../../ops/deploy/host.py) overwrites
`X-Forwarded-For` with `$remote_addr`; it does not append untrusted client text.
Loopback mode trusts local callers and must not be enabled silently. It does not
authenticate the real-world person behind an address or traverse arbitrary
proxy chains. The recorded origin is the selected observed IP, not an external
identity attestation.

### Preserve the bounded Owner query

The public event shape adds `context: null` for history or the exact typed object
for recorded new context. No separate outcome field is added: the four access
actions provide the outcome label. Historical business actions do not acquire
an invented access outcome. The existing `actor` filter matches the exact
recorded actor or exact recorded `context.user_id`; it performs no current-account
lookup and adds no query parameter. Action and resource filters remain exact.

The existing 262144-byte page text budget includes actor, action, resource and
the compact serialized context; null adds zero context bytes. Typed UUID/IP/source
validation also bounds individual context values. Keep the Owner authorization,
exact timestamp ordering, snapshot sequence, continuation checks, late-response
discard and per-page memory limit. Presentation distinguishes unavailable history
from new observed context and preserves separate whole-chain verification.

### Measure the durable record boundary

Start a monotonic timer immediately before waiting for the access sink and stop
after its successful commit. Include scheduling, writer/connection/lock waits,
append and commit. A failed append is a failed sample, not a fast successful one.
Do not time only `append_transaction`, which returns before its caller commits.
This measurement excludes the preceding HTTP/business calculation and does not
claim the latency of every earlier business-event transaction.

Follow [the matrix protocol](../access-audit-matrix.md): declare host, build,
database configuration, populated history, concurrency, enabled operations and
sample counts. Preserve individual samples with a request/operation reference,
commit status, maximum and distribution summary. Compare every successful sample
with the strict `<500 ms` criterion. Neither an average nor p95 substitutes for
that bound. Finite observed results apply to their declared conditions; no new
latency results are established by this ADR.

## Consequences

Business operations can retain their atomic audits without rewriting each HTTP
handler. The shared context adds correlation and stable identity where actually
observed; it does not change any role or resource policy. In particular, Client
document access remains denied pending its explicit human policy decision.

Every admitted product operation incurs another durable append and contention
on the shared chain. Existing request bounds also bound pending access writes.
Shutdown ownership, full-principal revocation, error/denial coverage, mixed
legacy/new integrity, data restoration and performance require direct evidence.
The Owner query and Qadra must understand the added context together.

Operational MFA logs, external chain-head anchoring, historical identity
reconstruction, external audit attestations and an expanded product catalog are
not provided by this decision. Local implementation, acceptance, integration
and deployment retain distinct evidence and status.
