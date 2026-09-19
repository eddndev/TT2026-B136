# Member access and durable authentication generations

## Status

Accepted design; implementation and acceptance are in progress. Executed evidence
belongs in `docs/verification-report.md`.

## Context

The single-office prototype creates active accounts with an initial password and
one-time MFA enrollment material. It supports case assignment, but operators need
a secret-free directory, an assignment selector and control of role and activity.
Procedural participants are separate records and must not become login accounts
by association.

Authentication reloads the current user from PostgreSQL. Activity checks alone
cannot permanently revoke a Redis session or MFA challenge: if the account is
reactivated before that value expires, the old value can become usable again.
PostgreSQL and Redis do not participate in a shared transaction.

## Decision

Add Owner-only member workflows over the existing `users` table. List and detail
return only identity, email, role, active state and a decimal revision string.
Queries have a bounded page size and UUID order; opaque cursors bind the filters
and, for assignments, the exact case and selection. Filter before pagination.
These pages are not a snapshot across separate requests.

An access command compares the expected revision and changes role and activity
together. Unchanged values at the current revision return the existing result;
stale revisions conflict even if the requested values now match. Every real
change increments both the shared user revision and an authentication generation.
The state, generation and audit event commit atomically. Credential material and
historical authors are not rewritten by this operation.

Store the generation alongside the principal in each Redis session and alongside
the user identity in each password-verified challenge. Compare it against the
current PostgreSQL user at MFA completion, session issuance and authentication.
Never replace an old challenge generation with a new one to issue a session.
Values from the old Redis representation lack a generation and are rejected;
deployment requires those users to log in again.

A committed PostgreSQL change invalidates earlier sessions and challenges without
Redis key enumeration or deletion. Reactivation requires a fresh password and MFA
flow. The generation is an internal authentication value, not a public directory
field. Recovery-code consumption still uses the shared user revision but does not
itself change access or invalidate unrelated sessions.

Use the common audited transaction lock to serialize account changes and creation.
Recheck the full acting Owner inside the transaction before reading a target or
changing it. Retain at least one active Owner, including concurrent attempts to
disable or downgrade the last two Owners. Database guards and restricted runtime
privileges protect the invariant and prevent arbitrary credential or generation
rewrites. Exhausted counters, stale revisions and audit failures leave no partial
access change.

Existing assignments remain when a user is disabled. Reactivation restores access
through those assignments only after a new login. Owners can inspect assigned
active and inactive accounts, select active unassigned accounts, and use the
existing assignment/removal mutations. Removing an inactive member is permitted.
Case closure does not prevent revoking access. Owners retain global case access
independently of assignment rows.

Qadra presents the real directory and assignment selectors. It distinguishes
account activity from case membership, explains retained assignments, and clears
the session after changing the acting account's access. Responses from an old
session or case cannot replace the current view. Lost mutation responses require
rereading current state before another decision; the UI does not blindly repeat a
command with a fabricated revision.

## Consequences

An unavailable Redis service does not undo a committed access change. It can
still prevent login or logout. Requests already authorized may finish; the
system cannot recall delivered bytes. Each mutation retains its final current
PostgreSQL authorization check.

Restoring PostgreSQL can restore an older authentication generation. Restoration
must therefore also invalidate sessions and challenges in the dedicated Redis
namespace before serving requests. A database restore alone cannot guarantee
that old authentication values remain revoked.

Account creation remains direct enrollment. Recoverable invitations, password and
MFA recovery, certificate authentication and the per-resource Client policy need
separate workflows and acceptance evidence. This change does not introduce billing,
multiple offices or legal identity certification.
