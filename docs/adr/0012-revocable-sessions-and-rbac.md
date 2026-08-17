# ADR-0012: Revocable sessions and runtime RBAC

- Status: Accepted
- Date: 2026-08-17

## Context

The local document API trusts `X-Actor` as an audit label and stores document
records on disk. It has no durable user identity, no session revocation, and no
runtime authorization. The existing cryptographic ports already provide
Argon2id password hashing, TOTP, recovery codes, AES-256-GCM, a clock, and an
append-only audit chain. The application design assigns durable identity data
to PostgreSQL and volatile session state to Redis.

JWT was considered because the written architecture names it. A self-contained
JWT would still require a Redis lookup to provide immediate logout and account
revocation. It would also duplicate the user's current role into a signed token
whose authorization data can become stale.

## Decision

Use opaque 256-bit random Bearer tokens. Redis stores only a SHA-256-derived
lookup key for each token, with the user principal as its value and a 24-hour
TTL. Logout deletes the lookup key immediately. Password-login challenges,
failed-attempt counters, and already-consumed TOTP codes also live in Redis
with bounded TTLs.

Store users in PostgreSQL with normalized email, Argon2id password hash, role,
active status, encrypted TOTP secret, hashed recovery-code slots, and an
optimistic revision. The TOTP secret is encrypted with AES-256-GCM under the
runtime KEK and authenticated with the user's UUID as additional data.

Expose bootstrap only while the user table is empty. Bootstrap creates the
first `owner`. Only an authenticated owner can create later users. Enrollment
returns the TOTP provisioning material and recovery codes once.

Remove `X-Actor` from the document contract. Protected handlers obtain the
actor from the Redis-backed session and check these permissions:

| Permission | Owner | Litigator | Paralegal | Client |
| --- | --- | --- | --- | --- |
| Create document | yes | yes | yes | no |
| Seal document | yes | yes | no | no |
| Verify document | yes | yes | yes | no |
| Export evidence | yes | yes | yes | no |
| Verify full audit chain | yes | no | no | no |
| Create user | yes | no | no | no |

Client document access remains denied until ownership or case membership is a
durable resource attribute. This avoids granting cross-client access based only
on knowledge of a document UUID.

Use the synchronous `postgres` and `redis` clients behind application ports.
The adapters serialize access internally and are suitable for this local
prototype. The HTTP boundary isolates every synchronous application call in
Tokio's blocking pool so database, file, and cryptographic work cannot block an
async worker.

## Consequences

- Every protected request can be revoked immediately and reloads the current
  active status and role from PostgreSQL.
- Passwords, plain recovery codes, plain session tokens, and plain TOTP secrets
  are never persisted.
- TOTP reuse inside the accepted clock-skew window is rejected through an
  atomic Redis claim.
- A rejected second factor consumes its password-verified challenge, limiting
  each challenge to one MFA attempt.
- Five consecutive password failures lock the email key for 15 minutes without
  revealing whether the account exists.
- PostgreSQL and Redis become required runtime dependencies for the secured
  application router.
- The implementation deliberately does not claim JWT conformance. Switching to
  signed JWT access tokens later remains possible behind the session port.
- Authorization is role based but not yet resource scoped. Client access to
  documents stays closed until case membership is implemented.
