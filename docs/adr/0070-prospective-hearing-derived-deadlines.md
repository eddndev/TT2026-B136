# ADR-0070: Prospective hearing results and configured derived deadlines

## Status

Proposed.

Application preparation, final capture, historical evidence restoration and the
PostgreSQL origin schema have focused verification. The complete compound
workflow is not yet implemented: atomic creation of the result and deadline,
durable replay, HTTP and Qadra integration remain pending. This
record establishes the proposed boundary; it asserts no successful acceptance,
performance result or deployment. It remains proposed until the complete
workflow and its evidence can be reviewed.

## Context

A declared ordinary hearing result can provide an exact temporal source for a
configured deadline. Recording a result and then submitting its consequence in
a second operation can leave only one side of the operator's reviewed intent.
The existing contextual resource flow addresses a related atomic-creation
problem, but its selected resource already exists. See
[ADR-0057](0057-atomic-resource-deadline-creation.md).

Here the result's first revision does not exist during preparation. The
[persisted hearing evidence](../../crates/application/src/deadlines/evidence_hearing.rs)
binds its actual recording time, author, administration and exact projections.
Constructing a fictional `HearingResultDetail` for preview would make that time
part of the deadline commitment before the transaction has recorded it.
Reusing a preview clock as the persisted time, dropping that field from existing
encodings or weakening receipt verification would conceal the difference.

The existing [result model](../../crates/application/src/hearing_results/model.rs)
has `values_digest` and a receipt with operation, action, expected revision and
`submission_digest`. It has no `capture_digest`. A compound commitment must not
invent that field or silently change existing result receipts.

## Decision

### Bounded, explicit compound instruction

Prepare one new ordinary `HearingResult` Record/R1 and one human deadline
Register/R1 in the same case. This is configured activation when the operator
records the result, not legal classification from narrative text. One consequence
per instruction bounds this interface; it does not assert that an act can create
only one legal term or prohibit separate legitimate deadline registrations.

Keep stable identities for the result, deadline and compound operation. Derive
the deadline source from this result's case, ordinary hearing, identity and R1;
reject an alternate source. An optional selected agreement must belong to those
exact proposed result values. Correction, withdrawal and technical reevaluation
are outside this creation instruction. Keep ordinary result-only registration
available with its existing contract.

Select an exact published profile, an exact calendar when applicable, a
responsible account and explicit tracking policies. Preserve their verified
selected captures and observed heads separately. Require the operator's
applicability statement, locator, condition answers, ordered quantity and
qualified temporal declaration when the selected profile requires them. Existing
hearing scheduling, optional continuation, admitted support and attendee
references remain exact, server-resolved evidence; the new result is prospective.

Occurrence or conclusion of a session does not identify its end time or an
ordered period. Do not infer those facts from text, use a maximum as the ordered
quantity, invent midnight or supply a time-zone fallback. Missing information
retains the evaluator's typed blocks or civil candidate. Malformed or foreign
material rejects preparation rather than becoming an acceptable blockage.

### Separate prospective commitment

Use a distinct preparation entry point, `prepare_hearing_derived_deadline`, for
the result command, human deadline command and verified existing material.
Return an immutable draft exposing the proposed result, calculation and
`review_digest`, with an exact-review check. An observation instant may support
validation; it is not the result's definitive `recorded_at`.

Validate the ordinary result draft and reuse pure trigger extraction and profile
arithmetic through a checked prospective source. Preserve the strict public
verification path for persisted deadline inputs. The prospective path must not
construct a stored result, source-event sequence, final deadline receipt or
persisted-source observation merely to satisfy an existing function signature.

The separate commitment binds all reviewed choices: result command and values,
exact scheduling and continuation evidence, support and attendees, administration,
actor, deadline identity and title, responsible, profile and calendar selections
and observations, qualifications, tracking policies and reproduced calculation.
It contains no invented persisted timestamp. A change to these choices must not
reuse the old approval. Existing HRES1/result receipt and deadline encodings
retain their meanings.

### Final capture and pending transaction

The storage confirmation must require both ManageHearingResult and ManageDeadline,
reauthenticate the same actor and revalidate membership, active case,
responsible and reviewed dependencies under the common audit lock. Preparation
alone neither authorizes a later write nor reserves a source revision.

Inside one transaction, obtain the real recording clock, persist and verify the
actual result R1, and run the normal tracked-deadline preparation against that
exact stored source. Compare the user-reviewed choices and prospective
calculation again. The final deadline receipt must bind the actual result detail;
a new compound receipt and immutable origin must link that final capture to the
approved prospective instruction. The review digest is not falsely presented as
the final deadline receipt.

`finalize_hearing_derived_deadline` implements this final calculation boundary in
the application layer. It rejects a result that differs from the reviewed
instruction, including author, administration, exact projections and offsets,
and requires an event reference matching that result's R1 and operation. It
constructs the ordinary tracked deadline from the supplied actual source and
compares the full calculation with the prospective review. Its immutable
`HearingDerivedDeadlineCreation` contains an HRDC1 commitment to that review,
the exact event, both ordinary receipts and the real capture time. Full result
evidence remains bound through the verified deadline capture. No HRES field or
historical deadline encoding changes.

This function neither writes to storage nor proves that the supplied event
exists in a database. The adapter must establish those facts in the transaction.
It is confirmation logic; historical origin reads must verify captured evidence
without invoking it to recalculate or overwrite the original calculation.

`restore_hearing_derived_deadline` verifies an untrusted historical evidence
envelope and returns an immutable record with the original HRDL1 and HRDC1 bytes.
It compares the result and deadline R1, authors, recording instants and offsets,
commands, captured source event, selected material and dependency observations.
The review commitment is reconstructed using the stored `DeadlineEvaluationRecord`;
no new `ProfiledDeadlineEvaluation`, evaluator call or finalizer is used during
restoration. Preparation and restoration share the same canonical encoders.

The origin must retain the actor's original role as well as identity and email:
HRDL1 binds that role, whereas the ordinary component receipts do not capture it.
The adapter must resolve each original selected revision and observed head, then
verify the origin, source-event row and audit chain. This record verifier neither
proves those rows exist nor authorizes their disclosure. Current account access
is checked separately and never substitutes today's role for captured authority.

The [origin migration](../../migrations/0032_hearing_derived_deadlines.sql) stores
the exact component identities, operations, R1 revisions, emitted event, original
actor role, canonical commitments and audit sequence. Unique compound slots
cover the result and deadline revisions; they do not make ordinary deadline
sources globally unique. The [row guards](../../migrations/0032_hearing_derived_deadlines_guards.sql)
require a currently authorized actor, matching components, the shared audit lock
and READ COMMITTED isolation. They compare the complete HRDC1 encoding, the
HRDL1 boundary projections and the exact audit marker. Updates, deletion and
truncation are rejected; runtime permissions allow only reading and named-column
insertion. These guards do not replace application validation of the full review.

The [startup inventory](../../crates/infrastructure/src/hearing_derived_deadline_schema/inventory.rs)
loads bounded batches and reconstructs full HRDL1 and HRDC1 bytes from verified
historical component revisions. It resolves the observed profile head from the
captured observation, verifies the source-event row and audit commitment, and
rejects compound audit markers without an origin. Strict catalog checks reject
changed columns, constraints, indexes, functions, triggers or elevated grants,
including authority reachable through transitive NOINHERIT roles. A fixture
which seeds consistent ordinary records and an origin proves this storage
boundary only; it does not prove atomic creation or authorize adoption of those
ordinary records by the future compound service.

Use the result's existing source event from
[the source-event migration](../../migrations/0015_deadline_source_events.sql).
Resolve its exact family, root, revision, operation and scope within the same
transaction. Do not emit another event, infer its identity from the largest
sequence, fabricate a cursor or mark it processed during creation. Result, event,
deadline, compound origin and audit records must commit together or roll back.
Sequence gaps after rollback remain valid.

Resolve an authorized exact compound origin before attempting fresh creation.
The ordinary Record operation is not itself a replay mechanism. A matching pair
of independently created records, or equal values without that origin, cannot
be adopted as compound success. Explicit reconciliation must return the original
R1 pair and receipts; a changed instruction conflicts. Current heads cannot
replace the historical origin, and no automatic write retry is introduced.

### Existing tracking and consumers retain their policies

Preserve the policy in [ADR-0037](0037-durable-deadline-reevaluation.md): a followed
source or profile change requires review; an eligible followed calendar change
may recalculate under its existing conditions. Creating a configured consequence
does not reapprove legal qualification after its source is corrected or withdrawn.
The dispatcher may later observe the already emitted result event; observing the
same source R1 must not create a duplicate deadline.

A blocked or merely civil result does not acquire an operational deadline.
Existing Agenda and Alerts may consume a committed deadline only after their
current authorization, acceptance and dependency-validity checks. No parallel
creation worker or invented technical author is required for this bounded action.

## Consequences

The design permits reviewing a calculation before its source has a recording
time while preserving full evidence in the eventual persistent deadline. It
requires a new prospective commitment and a later transaction-aware adapter;
joining two successful ordinary operations is not proof of atomic creation.

Application tests must distinguish absent persisted source from an actual source,
preserve explicit precision and blocking, and reject changed reviewed material.
Durability, concurrent reconciliation, rollback, restart and restore require
separate storage evidence. HTTP and desktop/mobile acceptance remain subsequent
work; no such result follows from an application-only draft.

This decision does not approve a legal profile or complete automatic activation
for notifications, resource acts or other sources. The resource-hearing family
in [ADR-0069](0069-resource-hearing-scheduling.md) remains distinct from ordinary
hearing results. Synthetic profiles can exercise this mechanism without becoming
approved legal rules or being silently published into an operational catalog.
