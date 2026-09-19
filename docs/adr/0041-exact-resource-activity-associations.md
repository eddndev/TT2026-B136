# Exact organizational associations between resources and activities

## Status

Accepted design; implementation and acceptance remain pending.

## Context

Resources and their declared acts already preserve exact historical resolutions,
documentary evidence and audited receipts. Hearing scheduling and calculated
deadlines have independent histories, permissions and operational rules. An
organizational relationship must not replace a historical receipt, change a
calculation, imply a legal effect or insert another ordinary case stage.

## Decision

Introduce a separate association root and immutable revisions. The initial Link
captures an exact resource revision, an optional exact declared-act revision and
one exact hearing or deadline revision. Its own operation, author, timestamp,
submission receipt and capture digest are audited with its write. Unlink appends
a reasoned organizational revision without deleting the association or mutating
either endpoint. A new distinct association may reference the same activity;
there is no inferred uniqueness by resolution, activity or resource family.

Resource and act captures are independently selected. An act reference includes
the resource revision that actually contains that act and its capture digest.
The service verifies both exact sources belong to the requested resource and
case. A correction of the act does not rewrite an older association. The
selected resource revision is distinct from the expected current resource head
checked during preparation and commit.

Owner may manage all cases; an assigned Litigator may manage an accessible case;
an assigned Paralegal may read; Client is denied. Authentication precedes source
lookup and the complete principal is checked again before releasing a read or
committing a prepared mutation. The persistence transaction rechecks the active
account, membership, active case, association head, resource head and exact
captures under the shared audit lock. New links require an active resource head.
Unlink is also allowed when that head is organizationally archived. Closed cases
remain readable and reject new mutations. Exact authorized operation replay
returns the original receipt without another write, including after closure.

The `RASL1` canonical selection binds typed identities, revisions, digests and
the optional-act discriminant. A hearing reference binds its submission digest;
a deadline and resource reference bind their capture digests. `RATX1` binds the
reviewed association, actor, observed administration and resource head, previous
receipt and reason. `RACP1` binds the definitive commit timestamp. Existing source
receipt validators verify referenced captures; caller-supplied hashes alone are
never proof of integrity. Linking does not admit document content again.

Current target information is separate from the historical association receipt.
Authorized list and detail reads resolve both under one transaction. A deadline's
operational projection is calculated from its real current head and current
observations, with a common checked-at instant. The selected old deadline is
preserved separately and cannot supply a current due date. History and mutation
receipts contain historical evidence and do not assert operational currentness.

## Consequences

Association creation and audit can roll back together without changing the
resource, declared act, hearing, deadline or case stage. Unlink and resource
archive do not cancel a hearing, retire a deadline, record attention or resolve
alerts. Existing activity identity and alert occurrence rules remain unchanged,
so several associations do not create duplicate notifications.

The first contract links existing activities. A resource scheduling context and
atomic creation of a new activity with its association require separate changes
to their existing transactions. This decision introduces no new temporal source,
legal profile, numerical rule or automatic activation. PostgreSQL permissions,
concurrency, rollback and restore require their own reproduced acceptance;
mocked application tests do not prove those persistence properties.
