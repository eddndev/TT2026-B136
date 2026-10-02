# ADR-0064: Absolute session lifetime and explicit activity

- Status: Accepted
- Date: 2026-10-02

## Context

The revocable sessions in [ADR-0012](0012-revocable-sessions-and-rbac.md)
expire after 24 hours. An optional inactivity limit needs an authoritative
server deadline: background reads must not keep a session alive, and a delayed
request must not recreate an expired or revoked session. Re-entry without
losing unsaved work also requires a browser draft boundary; a backend timeout
alone does not provide that user journey.

## Decision

Keep `absolute_only` as the default: 86,400 seconds after issuance, without
an inactivity deadline. The application validates a positive absolute lifetime
of at most 86,400 seconds and an optional positive idle lifetime no longer than
the absolute lifetime. `serve --session-idle-seconds` or
`TT_SESSION_IDLE_SECONDS` opts into idle expiration; absent configuration never
invents a duration. Invalid configuration prevents startup. The deployed
application keeps the default until browser re-entry and draft preservation
are ready and an operational idle duration is selected.

Redis stores a versioned hash under the SHA-256-derived token key. Its eight
fields hold the format version, opaque identity JSON, selected absolute and
idle durations, creation and last-activity times, and both deadlines. Times
are integer Unix milliseconds. Zero represents the absent idle duration and
deadline internally; public responses use `null`. Rust strictly decodes the
identity, including the authentication generation. Lua never converts that
generation to a floating-point number.

Create, read, and activity operations use atomic Lua scripts and Redis time.
They validate the complete schema, policy, time relationships, positive TTL,
and exact expiration date against the earlier deadline. `PEXPIRETIME` requires
Redis 7 or later; the backup snapshot contract separately requires Redis 7.4.
Legacy values, missing expiration, malformed state, and a different configured
policy require reauthentication. They are not migrated by assigning new time.

`GET /api/v1/auth/session` returns the current principal, policy, server time,
and deadlines. `POST /api/v1/auth/activity` accepts an empty authenticated
request and advances only idle time, capped by the original absolute deadline.
Neither route accepts caller-selected time, identity, policy, body, or query
parameters. Absolute-only activity leaves stored deadlines unchanged. Normal
authorization, `/me`, polling, and status reads never record activity.

Admission reads the session, revalidates the active PostgreSQL user and full
principal/authentication generation, then confirms the same live session in
Redis. Activity performs one final atomic comparison before updating the
existing key. A missing, expired, or revoked key remains absent. Both MFA paths
return the same public metadata and retain their existing audit-failure cleanup.

The independent browser monitor uses monotonic elapsed time and subtracts the
complete round trip from the received server window. It retains the most
conservative server-to-monotonic conversion during a login, so a faster later
reply cannot recover time already deducted. Explicit confirmed activity may
advance only the idle deadline. A single local expiry timer issues no network
requests; rechecks read status and activity calls require an explicit signal.
Failed transport retains the last confirmed deadline, while rejection or invalid
metadata expires the local monitor. Integration with protected views and draft
editors remains separate.

## Consequences

- Existing legacy sessions require a new login after the format change.
  All application instances sharing session keys must use a compatible format
  and policy. There is no automatic conversion during an upgrade or rollback.
- Only explicit activity can extend idle time; it never extends absolute time.
  The endpoint is a client activity signal, not proof of human presence.
- Redis errors do not authorize access or create a local fallback deadline.
  Public responses never expose authentication generations or a replacement
  bearer token from status/activity calls.
- PostgreSQL and Redis do not share a transaction. A concurrent account change
  after its durable read can overlap a Redis activity update; the changed
  generation still prevents subsequent authorization. A mutation admitted
  before expiration may finish an already-started transaction.
- This backend contract does not preserve drafts, activate browser timers, or
  complete the re-entry experience. It supplies deadlines for that separate
  client integration. No numeric idle duration is approved by this decision.
