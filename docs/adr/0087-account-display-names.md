# 0087 - Account display names

## Status

Accepted. This record supersedes the statements of
[ADR 0075](0075-recoverable-member-invitations.md) that "no additional personal
profile is required" and that invitations introduce no "extra personal
fields". ADR 0075 is not edited; every other part of it remains in force.

## Context

Office accounts are identified to people only by their email address. The
overview greeting and the account menu therefore show an address where a
person expects to read a name. The project authors decided that every office
account carries one human display name, written by the Owner when the account
is created, and that the name appears only in the overview greeting and the
account menu.

The session identity (`Principal`) is compared as a whole value in many
places: the Redis session record, certificate session provenance, MFA
admission and every session-validity check. Adding a field to it would change
those stored formats and invalidate every live session on deploy.

The certificate authority keeps a separately provisioned identity store
(`qadra_ca`) with its own operator account. That store has no use for a
display name.

## Decision

Naming. Storage and Rust use `display_name` (`DisplayName` for the validated
type). Every JSON key on the wire is `name`, in requests (bootstrap, direct
member creation, invitation issue) and in the session user object of responses.
The session user object is `{id, email, name, role}` in that order, returned by
`GET /auth/me` at the top level and under `user` by the enrollment, MFA,
`GET /auth/session` and `POST /auth/activity` responses. Login requests do not
accept `name`. Invitation summaries, pages, enrollment preparation and receipts
do not return it, nor do the member directory and case member pages.

Errors. An invalid name answers `422 invalid_display_name` on bootstrap and
direct member creation, and `400 invalid_invitation_request` on invitation
issue, following each module's existing convention. A body without `name` is
rejected by the JSON reader (`422` for the identity routes, `400 invalid_json`
for invitations).

Validation is a domain rule (`crates/domain/src/identity/display_name.rs`):

1. Reject the value if any character is a control character (general category
   Cc), a format character (general category Cf), U+2028 LINE SEPARATOR or
   U+2029 PARAGRAPH SEPARATOR. This check runs before trimming, so such
   characters cannot hide at the edges of a value. The two separators are
   rejected because they break the single-line greeting. The Cf table was
   generated from Unicode 16.0.0 (Python `unicodedata`); characters assigned
   to Cf by later Unicode versions are accepted until the table is
   regenerated.
2. Trim with Rust `str::trim` (Unicode White_Space).
3. Require 1 to 120 Unicode scalar values.

There is no normalization and no uniqueness rule. A stored value must already
be canonical (equal to its own validated form); a non-canonical stored value is
a storage fault, not something to repair silently.

Who writes it. The Owner writes the name at the first-Owner bootstrap, at
direct member creation (any role, including Client) and at invitation issue.
The invitee never types it. For invitations the name is stored on the
invitation row, is part of issue idempotency (the same operation id with a
different name is a concurrent modification) and becomes the account name when
the invitation is accepted. Resending an invitation never changes it.

Immutability. The name is fixed at creation and there is no rename flow. The
runtime database role receives INSERT but never UPDATE on the column, for both
users and invitations; startup validation rejects a role that could update it.

Principal. The name is kept out of `Principal`. It travels on `UserRecord`,
`EnrollmentResult`, `SessionStatus` and a separate `CurrentAccount` read, and
reaches clients as the `name` field of the session user object. Session
records, certificate provenance and whole-Principal comparisons are unchanged,
so no session is invalidated by this change.

Audit. The hash-chained audit log keeps recording actor and subject emails
exactly as before; the display name is never written to any audit event.

Certificate authority. `qadra_ca` gets no name column. When the user
repository reads the CA store it fills the required field with the fixed label
`CA operator`, which is never stored and never returned: the CA HTTP layer
builds its own operator identity from id and email.

Existing rows. The migration backfills existing users and invitations with
the local part of their email (trimmed, cut to 120 characters, trimmed again so
the cut cannot end in a space, `cuenta` when empty) and then enforces NOT NULL
and the length and trimming CHECK, which PostgreSQL stores as
`((display_name = btrim(display_name)) AND ((char_length(display_name) >= 1) AND (char_length(display_name) <= 120)))`.
Repeating the migration changes nothing. Users and invitations need two
migration files because the member family runs before the invitation family.
Invitation operations recorded before the migration carry no name: replaying
such an issue is a concurrent modification (409), and replaying such a resend
fails as a storage error because its stored response cannot be decoded.

Bootstrap tooling. There is no CLI bootstrap command; the first Owner is
created over HTTP, and the documented curl recipes and demo scripts send
`name` with the email and password.

## Consequences

- The overview greeting and the account menu can greet a person by name.
  Contracts, the team directory, workload, pickers and reports keep showing
  the email.
- Clients that validate the session user object with a closed key set must
  accept `name`.
- A wrong name can only be corrected by an administrator outside the
  application until a rename flow is decided.
- The database CHECK cannot express the control and format rule, so the
  application validates stored values on read and the startup inventory
  rejects a database holding a non-canonical account or invitation name.
- Rolling back to a binary that predates the column requires the
  pre-migration backup, because older binaries reject the extra column.
