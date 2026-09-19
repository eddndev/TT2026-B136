# Member directory and account access API

## Delivery status

The backend and Qadra implementation passed focused checks; integrated acceptance and CI closure are pending. This document is not
acceptance evidence; executed checks belong in [the verification report](verification-report.md).
The rationale is [ADR-0043](adr/0043-member-access-and-authentication-generation.md).
All paths below start with `/api/v1` and require an authenticated active Owner.
Client permissions on documents and other case resources remain unchanged.

## Directory and assignment selectors

| Method and path | Query or body | Response |
| --- | --- | --- |
| `GET /users` | `limit`, `status`, `role`, `email_prefix`, `cursor` | User page |
| `GET /users/{id}` | None | User summary |
| `PUT /users/{id}/access` | Access change | Confirmed user summary |
| `GET /cases/{case}/members` | `limit`, `selection`, `role`, `email_prefix`, `cursor` | Case member page |
| `PUT /cases/{case}/members/{user}` | Existing empty body | Existing assignment operation |
| `DELETE /cases/{case}/members/{user}` | Existing empty body | Existing removal operation |

`POST /users` remains the existing direct account enrollment operation. It is not
an invitation, acceptance link or password recovery flow. The directory never
returns password hashes, MFA enrollment material, recovery codes, tokens or the
internal authentication generation.

User summary:

```json
{
  "id": "00000000-0000-0000-0000-000000000007",
  "email": "staff@example.com",
  "role": "paralegal",
  "active": true,
  "revision": "0"
}
```

Roles are `owner`, `litigator`, `paralegal` and `client`. Revisions are canonical
nonnegative decimal strings at most `9223372036854775807`. Numeric JSON values,
leading zeroes other than `"0"`, signs and whitespace are rejected.

User pages contain `items`, `has_more` and `next_cursor`. Case pages additionally
contain `case_id`; each item has the user summary fields plus `assigned_at`, an
RFC3339 UTC timestamp for assigned accounts or null for available accounts.
An empty terminal page uses `has_more:false,next_cursor:null`. No total is implied.

`limit` defaults to 50 and accepts 1 through 100. Directory `status` defaults to
`active` and accepts `active`, `inactive` or `all`. Case `selection` defaults to
`assigned` and accepts `assigned` or `available`. Assigned pages include inactive
accounts; available pages include active accounts that are not assigned. Owner
access is global regardless of whether an assignment row exists.

Optional `role` uses the four role values. Optional `email_prefix` is ASCII,
trimmed and lowercased, at most 254 bytes and without controls. Empty prefixes
mean no filter. Percent, underscore and backslash are literal prefix characters,
not client-selected SQL wildcards.

Filtering precedes pagination. Rows are ordered by UUID ascending. The cursor is
opaque ASCII of at most 768 bytes; it binds the last identity and normalized
filters, including case and selection. It may be reused with a different valid
page size. Changing filters requires starting a new page sequence. A page sequence
does not promise a transaction snapshot across concurrent account changes.
Unknown or repeated query fields are rejected. Responses have `Cache-Control: no-store`.

## Access changes

```json
{
  "expected_revision": "0",
  "role": "litigator",
  "active": false
}
```

The body is closed: all three fields are required and additional fields are
rejected. Role and activity change together. The expected revision also competes
with recovery-code consumption, which can legitimately make a displayed revision
stale. The operator must reload before confirming another decision.

A real change increments the user revision and internal authentication generation
and commits its audit event atomically. Earlier sessions and login challenges
remain invalid after reactivation. A fresh password and MFA login is required.
Inactive accounts retain their assignments and historical authorship; disabling
does not delete or reassign a deadline. Reactivation restores access through
retained assignments after a new login.

At the current revision, requesting the same role and activity returns the
existing summary without another update or audit event. Replaying an earlier
revision after a real change conflicts, even if its requested values now match.
There is no automatic retry with an inferred new revision. After an uncertain
response, reload the current account and let the operator confirm any remaining
change. Do not report a failed mutation merely because a successful self-change
invalidated the calling session; the confirmed response is returned first.

At least one active Owner must remain. Self-deactivation or self-demotion is
permitted only if another active Owner remains. Qadra clears its session after a
real change to the acting account. Removing membership is permitted for inactive
accounts and closed cases; account deactivation preserves the membership rows.

## Authorization, audit and errors

Owner authorization precedes target/case lookup and filtering in the application
and again inside the PostgreSQL transaction. Read events are committed before
rows are disclosed; read responses also require complete principal reauthentication.
Every mutation revalidates the actor under the common audited lock. Its current
projection is returned after commit, including for an authorized self-change.

| Condition | HTTP status and code |
| --- | --- |
| Missing, expired or invalidated session | 401, identity error |
| Authenticated non-Owner | 403, `permission_denied` |
| Missing account or case visible to Owner | 404, `user_not_found` or `case_not_found` |
| Invalid query or access body | 400, `invalid_input` |
| Stale revision | 409, `user_revision_conflict` |
| Last active Owner would be removed | 409, `last_active_owner` |
| A real change would overflow revision or generation | 409, `user_access_version_exhausted` |
| Invalid stored response, database or audit failure | Generic server error without sensitive details |

The activity state, role, generation and audit cannot commit partially. Redis
availability is not required to confirm a PostgreSQL access change; obsolete
Redis values are rejected by generation comparison and expire normally. Legacy
values without generation require new login. Restoring older PostgreSQL state
requires invalidating the dedicated Redis sessions and challenges before serving
traffic; generation comparison alone does not prevent revival after rollback.
