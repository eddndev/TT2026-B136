# Bounded Owner consultation of existing audit events

## Status

Implemented and accepted locally with application, storage, HTTP, UI,
integrated API/restore and real-browser verification. CI for this increment,
merge and activation remain pending. See `docs/audit-events-api.md`.

## Context

The audit log already records a hash-chained sequence of timestamps, actors,
operations and resources. Runtime database privileges allow append and read,
not replacement or deletion. The HTTP workflow verifies the whole chain, while
the CLI can load the complete local log. Neither is a bounded, authenticated
activity query for the Owner interface.

Historical actors and resources are recorded strings. Some events use an email,
some a UUID rendered as text, and background activity may use `system`. There is
no dedicated actor UUID or source IP in the chained event representation. Those
values cannot be reconstructed reliably from a current account or request.

Canonical UTC timestamp text preserves nanoseconds. Lexicographic ordering of
RFC3339 strings with different fractional precision is not chronological; a
PostgreSQL timestamptz conversion can round nanoseconds to microseconds. Query
support must preserve original timestamp text and canonical hash bytes.

## Decision

Add a separate Owner-only application workflow and an HTTP event query. Keep
whole-chain verification and the historical CLI unchanged. Every page checks a
current bearer identity, an active persisted Owner with that full identity, and
the same authenticated Principal again before disclosure. A continuation cursor
never grants access. Empty pages follow the same authorization rules.

Queries require an inclusive start and exclusive end, at most 366 exact days.
Optional actor, operation and resource filters match the recorded UTF-8 strings
exactly. Input limits bound filtering and URLs, without imposing new per-field
limits on valid historical records. A page contains 1..100 requested positions,
with at most 262144 total UTF-8 bytes across returned actor/action/resource text.
A capacity excess rejects the complete response with a typed error; it does not
truncate an event. Storage measures text lengths before fetching large values.

Order by exact UTC seconds, nanoseconds and sequence. Persist an indexed exact
query projection without rewriting the historical timestamp or changing event
canonicalization. The first page observes the greatest existing sequence under
the audited transaction lock, before recording the read. Subsequent pages retain
that maximum and seek strictly after the previous chronological tuple. New
appends, even backdated ones, stay outside that pagination snapshot.

The canonical cursor records the observed maximum, last tuple and exact filters.
Its unsigned, versioned ASCII representation is deliberately transparent and
strictly validated; it is selection state, not an integrity attestation. Reject
noncanonical fields, changed filters and sequences outside PostgreSQL's signed
64-bit range. Return sequences as decimal strings over JSON so browser clients
do not lose precision. Changing only the requested page length is permitted.

The store applies filters and keyset bounds before fetching at most limit plus
one positions. It then records `audit.events_read` within the same successful
transaction; its append occurs after the snapshot and is not chased by the
current pagination. Denied, malformed or capacity-failed storage operations
roll back. An application reauthorization failure suppresses disclosure even if
the already-completed successful database read has left its audit event.

The Qadra Auditoria page displays existing timestamp, recorded actor, operation
and resource fields as escaped text. Keep the separate Verify chain action.
Clear private records on logout or permission loss, and reset pagination when
filters or the explicit refresh action change. Do not infer clickable resource
routes, claim that a filtered page verifies the whole chain, or fabricate IPs.

## Consequences

A bounded read no longer needs to load the complete log into application memory.
A snapshot maximum stabilizes pagination across appends but is not a database
backup or a cryptographic proof of an externally anchored head. The existing
truncation limitation in `docs/adr/0007-audit-chain-anchoring.md` remains.

Event capture with stable user UUID and trusted source IP remains a separate
cross-cutting change: it needs rules for HTTP, CLI and background actors, and a
versioned chained representation. No historical backfill is inferred. Append
latency claims require measurement independent from this query acceptance.

Acceptance covers exact sub-microsecond ordering and boundaries, populated
migration, cursor stability, concurrent appends, append-only runtime privileges,
full-principal revocation, capacity rejection, escaped display and preservation
through backup/restore. Focused local results are recorded separately in
`docs/verification-report.md`. Integrated API/restore passed with exit zero in
376.459967 s; two real-browser scenarios passed in 13.7 s of Playwright time
(185.7532 s for the complete command). Both campaigns preserved all 3198
source-inventory files unchanged. These are local acceptance results,
not evidence of CI, merge, deployment or durable append latency below 500 ms.
