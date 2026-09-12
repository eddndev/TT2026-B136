# ADR-0014: Case metadata and current membership

## Status

Accepted.

## Context

The authenticated HTTP application persists users in PostgreSQL and sessions
in Redis, but documents still use the independent local workflow described in
`docs/adr/0011-local-document-workflow.md`. Global roles alone cannot isolate
case metadata. Storing memberships in bearer sessions would allow a removed
member to retain access until that session expires.

## Decision

Add cases and explicit user assignments in PostgreSQL. A case has a random UUID,
a required title (at most 200 Unicode scalar values), a required reference (at
most 100), and the authenticated creator's user UUID. Reject control characters
and trim surrounding whitespace. The reference is a human-facing label; it is
not unique, a legal identifier validation, or an authorization credential.

Owners may create and read every case and are the only membership managers.
Litigators may create cases and receive an initial assignment. Litigators,
paralegals, and clients may read metadata only while assigned. Creating a case
does not give a litigator authority to assign other users. Removing the creator's
assignment removes their visibility if they are not an owner. Owners retain
global administrative access even without an assignment.

`CaseService` authenticates the bearer token through `IdentityWorkflow` on every
operation. That workflow reloads the active user and current role. The delivery
adapter cannot supply an actor, role, or visibility scope. The application
selects all-case or assigned-user scope, and the repository evaluates membership
within the same SQL query as the list or detail read. Filtering precedes
pagination. UUID ordering gives a deterministic order for a fixed set; pagination
is not a snapshot across concurrent writes. The default page size is 50 and the
allowed range is 1 through 100.

Hidden and nonexistent case details both return `404 case_not_found`. Member
mutations require an owner before examining the resource. Adding an existing
active user and removing an assignment are idempotent. An unknown case fails;
an unknown or inactive target fails on assignment. Assignments have foreign keys
and a composite primary key. Case creation and its initial creator assignment
share a PostgreSQL transaction, so either both persist or neither does.

Membership is never cached in Redis. A read starting after a successful removal
observes the removal with the same session. An already-running read may complete
under its earlier database snapshot; revocation cannot retract an earlier
response. Concurrent addition and removal follow database ordering, so a later
explicit addition can restore access.

`migrations/0002_cases.sql` follows the identity migration. The case repository
applies both idempotently at connection time. CI and
`scripts/test-backends.sh` use a separate database for case integration tests so
they cannot invalidate the identity bootstrap tests' empty-user precondition.

## Consequences

- Membership isolates case metadata, including list results and direct UUID
  access. Real PostgreSQL tests and `scripts/api-demo.sh` demonstrate that
  isolation and removal using a still-valid Redis session.
- Clients remain denied every document permission. Documents are not yet bound
  to cases; assigning a client does not grant upload, sealing, verification, or
  evidence export. The existing staff document routes still use global roles.
- Case metadata is stored as PostgreSQL text, not encrypted with the document
  envelope. Database access and backups must be protected before deployment.
- Assignment and case creation do not append to the separate file audit log.
  Durable case mutation history and atomic document/audit persistence need a
  shared transactional design; this change does not imply that guarantee.
- This model covers case titles, references, and user access assignments.
  Procedural participants, hearings, deadlines, lifecycle changes, document
  associations, document history, and the product interface remain separate work.
- PostgreSQL access remains synchronous and serialized per adapter connection;
  HTTP runs it on Tokio's blocking pool. Production pooling, TLS, workload limits,
  and the signing threat model remain open as described by
  `docs/adr/0012-revocable-sessions-and-rbac.md` and
  `docs/adr/0002-rsa-signing-crate-and-advisory.md`.
