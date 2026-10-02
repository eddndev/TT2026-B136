# Session re-entry and in-memory editor drafts

## Status

Accepted for the browser session boundary and explicitly adapted editors.
An editor is not recoverable merely because the registry exists. Operational
idle expiration remains disabled until the required editor inventory is accepted.

## Context

`docs/adr/0064-explicit-session-activity.md` defines server-authoritative absolute
and optional idle deadlines. A browser must stop admitting business requests at
the confirmed deadline, even without a further HTTP response. Hiding a page can
also defer timers; returning to it requires a fresh session read before business
requests resume. Neither visibility nor background reads represent user activity.

Unmounting the protected application removes dialogs, old API scopes and private
query results. It would also discard unsaved form values unless those values are
captured first. Keeping the entire component tree would retain authorization
assumptions, pending callbacks, private rows and credentials across authentication.

A fresh MFA for the same person is not proof that the person still has access to
a particular case or may replace its current revision. A timed-out mutation may
already have committed. Re-entry cannot safely resubmit it automatically.

## Decision

The application composes the existing session monitor with a lifecycle coordinator
and a draft registry that live outside the authenticated component tree.

- The lifecycle closes request admission, captures registered drafts synchronously,
  invalidates the local bearer and then unmounts the protected application.
- Only trusted pointer, keyboard or input events can request bounded activity.
  Reads, timers, focus and visibility never renew an idle deadline.
- A hidden page stops business admission. Returning uses a distinct session read,
  waiting for any previous activity operation without treating it as confirmation.
  A network failure leaves admission closed; an explicit retry or expiration
  resolves that state. No retry loop is introduced.
- Open modal dialogs temporarily leave the top layer during a visibility check.
  This preserves their values and makes session controls reachable; an ancestor's
  `inert` attribute alone does not block a modal dialog. A successful confirmation
  can reopen the same surviving dialogs. Expiration discards those DOM references.
- Logout discards local drafts and access immediately. Remote revocation failure
  is reported separately and cannot reopen the application or affect a later MFA.

A draft descriptor names the exact principal, context, resource, action and editor
instance, with schema and original base revision. Nested editors additionally name
their owning draft, structural field path and stable row identity. Closing an
owner removes its descendants. Array positions and display labels are not identity.

Each adapter supplies an explicit allowlist of values and a synchronous capture.
The registry preserves incomplete text, ordering, undefined values, non-finite
numbers and selected File/Blob data. It rejects runtime objects and secret fields.
No draft is written to browser storage or uploaded as a recovery mechanism.
API clients, promises, permission flags, cached private rows and callback functions
are reconstructed after authentication instead of entering the capture.

A same-principal MFA makes descriptors available without automatically restoring
an editor. A different principal or explicit logout discards them. Applying values
requires new authorization for the destination and action. The asynchronous
admission is tied to the current principal and registry generation; expiration,
closure or revocation makes a pending result unusable. The adapter stays blocked
until application succeeds. A failed read retains its capture for an explicit retry.

Resource denial removes only the affected editor. A proven case denial removes
that case's drafts. A closed case or changed revision is not a loss of read access:
authorized comparisons preserve the original values and require a new decision
before submission. An uncertain earlier submission must be reconciled with current
records or its exact operation receipt before a further explicit command.

## Consequences

The design preserves the existing Qadra components and separates browser recovery
from server authorization. Every mutation still checks current server permissions,
state and revision. Browser recovery neither extends an absolute deadline nor
provides a new authentication factor.

Recovery lasts only while the same tab remains open. Closing or reloading it loses
unsaved values and selected files. Enrollment secrets and passwords are intentionally
excluded. A capture failure still closes access and reports that some changes could
not be preserved. Each additional editor requires its own adapter and acceptance;
the registry is not evidence of universal draft recovery.
