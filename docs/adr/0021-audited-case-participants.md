# Case participant directory with immutable, audited revisions

## Status

Accepted.

## Context

Case membership in `docs/adr/0014-case-membership-and-isolation.md` controls
authenticated access. A directory must also describe people or authorities
associated with a case without creating accounts or granting access. A procedural
label, a name and an account role have different meanings. A directory must not
infer that two matching names identify the same person.

Concurrent editing and organizational archiving must preserve both current
values and attribution. The transactional audit boundary established in
`docs/adr/0016-case-document-transactions.md` also applies to these records.

## Decision

### Identity, values and authority

A participant has its own UUID and one immutable case association. It does not
reference an account as its represented identity. Captured authors separately
reference the authenticated account that performed each change. There is no
global person matching or uniqueness constraint on a participant name and role.

Values consist of required `display_name` and `procedural_role`, optional
`organization` and `legal_status`, and `directory_status` (`active` or `archived`).
The text records organizational information supplied by an authorized user; it
does not validate identity, judicial appointment, legal representation or a
procedural transition. The status controls the directory's active/archived view,
independently of any case or legal status.

The four text limits are respectively 200, 80, 200 and 160 Unicode scalar values.
Control characters are rejected before Unicode whitespace trimming. Empty
optional values become absent; required values remain nonempty. Interior spaces,
case, punctuation, accents and Unicode normalization forms are preserved.
Passwords, private keys, contact details and legal identifiers are not fields of
this directory. Its values and historical actor email are stored as authorized
PostgreSQL metadata, not as encrypted document content.

Owner can read and manage all case directories. Assigned Litigator can read and
manage; assigned Paralegal can read. Client has neither permission. Existing
Client access to assigned basic case metadata is unchanged. A manual procedural
role never changes account privileges. Service authentication precedes storage;
the transaction independently reloads the active actor, role and case membership.
An item must match its immutable case even when the actor is Owner.

### Canonical values and provenance

`ParticipantValues` is a validated domain value with private fields. Its `PART1`
encoding contains the five ASCII prefix bytes, then:

1. Name and role, each as a four-byte big-endian UTF-8 byte length and its bytes.
2. Organization and legal status, each as byte zero for absence, or byte one
   followed by a four-byte UTF-8 length and bytes.
3. One state byte: zero for active, one for archived.

The maximum encoding is 2584 bytes. SHA-256 covers these values only, not identity,
revision, actor or time. For `Ana`, `Witness`, absent optionals and active status,
the canonical hex is `504152543100000003416e61000000075769746e657373000000` and
the digest is `fc4e619a010411b046c79cbd60bfd6292c9948cbef2d5caea6ae2c17a88263ad`.
Fixed ASCII and Unicode vectors connect domain, application and SQL validation.

Each immutable revision captures the digest, normalized values, actor UUID,
actor email and UTC timestamp. Historical authors are not reconstructed by
joining their current user profile. The clock records the operation instant;
revision and audit sequence order commits, so timestamps are not promised to
increase under concurrency.

### Transactional persistence

Creation stores an immutable root, active revision one and one audit event in a
single transaction. A deferred reference requires the root's first revision at
commit. Revisions run from one through `u32::MAX`, contiguously. SQL also enforces
active status at revision one, valid canonical fields/digest, actor references
and immutable roots and history. Startup checks schema, runtime privileges and
inventory. Stored corruption yields an internal failure, never caller validation.

Every operation acquires the same exclusive advisory transaction lock used by
the audit chain. Scope and authority are checked before revealing the current
revision or checking a conflict. Mutation uses an expected revision, appends the
successor and its event, and returns the projection committed by that operation.
It does not issue a follow-up read. Any insertion, audit or commit failure rolls
back the complete mutation. Reads, lists and history commit their audit event
before returning values, including empty pages.

Full replacement deliberately replaces all values. A separate status-only
operation reads current values and changes only `directory_status` inside the
same transaction. It cannot be composed from an external read followed by full
replacement: that could overwrite a concurrent text edit. Both operations append
a revision even when accepted values equal their predecessor. A stale expected
revision and an exhausted counter have separate conflict codes. Neither produces
a success event or returns an unauthorized head.

Events are `participant.created`, `participant.updated`,
`participant.directory_status_changed`, `participant.listed`, `participant.read`
and `participant.history_listed`. Snapshot resources bind case, participant,
revision and digest; list/history resources bind their case/item scope. Free-form
participant names and legal labels are not copied into event resources.

Runtime privileges permit SELECT/INSERT and the required pure validation
functions, without UPDATE/DELETE/TRUNCATE or ownership. PostgreSQL must use UTF-8.
Project references inside SQL functions bind to the installation schema so
validation also works with the empty search path used by restore. These checks
do not protect against an administrator rewriting a complete coherent database;
`docs/adr/0007-audit-chain-anchoring.md` remains applicable.

### Queries and browser behavior

Current queries select each participant's latest revision before applying all
filters and pagination. Name is a literal case-sensitive substring; procedural
role is exact and case-sensitive. Comparisons do not depend on locale-sensitive
case folding. Status defaults to active and can explicitly include archived or
all rows. The ascending UUID cursor is exclusive; history uses an exclusive
descending revision cursor. Limits are 1 to 100 and default to 50. Cursors are
returned only when an additional row exists. No global total or common snapshot
across separate page requests is promised.

Strict JSON rejects repeated/unknown fields and is bounded to 8 KiB, including
trailing bytes. Raw revisions are parsed as unsigned integers before semantic
validation, so zero is a validation error rather than malformed JSON. The maximum
valid text values fit the limit even with escaped astral Unicode. HTTP shares
the existing global admission and blocking-work budget.

Qadra places Documentos and Participantes within the selected case. It reuses the
original visual system, distinguishes account access from participant registration
and exposes current provenance and immutable history. On replacement conflict it
preserves the draft, presents current values and requires explicit resubmission.
A status conflict preserves only the intended state change and requires another
confirmation against current values. Scope changes invalidate pending results;
access denial clears sensitive values and drafts. No automatic retry converts
an uncertain creation into a duplicate participant.

### Boundaries with legal identity

The directory is a foundation of the participant use case in the versioned
analysis and design, not evidence that every criterion is fulfilled. Verified
identity, typed identifiers, verified-person/role duplicate detection, FIREL and
active penal-case conditions remain open in `docs/product-completion.md`.

The [CNPP, article 105](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf#page=28)
distinguishes procedural subjects and parties. Its terminology is not equivalent
to an arbitrary directory label or the application's access matrix. The
[OAJ FIREL agreement, articles 6 and 7](https://sidof.segob.gob.mx/notas/docFuente/5797197)
regulates electronic judicial signing and distinguishes possession of a
certificate from procedural authorization. These sources support keeping those
concepts separate; they do not validate a directory entry. No obligation to
collect a judge's FIREL merely to record an internal directory entry was located
in this bounded review. This finding does not remove the product's approved
criterion or establish that no other applicable rule exists.

## Consequences

- Case staff gain an attributable directory without creating user accounts,
  cross-case person matching or legal permissions from free text.
- Archive/reactivation preserves all history and does not overwrite concurrent
  text changes without an explicit full replacement.
- Migration adds an empty directory to existing cases. It neither invents
  participants from user assignments nor changes document evidence.
- Backups must preserve roots, all revisions, captured actors, case references
  and audit history together. Restore compares complete rows and retains the
  existing encrypted document versions and evidence ZIPs.
- Historical text and actor emails remain visible to authorized directory
  readers. Operational privacy, retention and judicial interoperability require
  their own policies and validation before a production claim.
