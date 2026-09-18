# ADR-0036: Captured deadline evaluations and independent attention

## Status

Accepted. The application contract, PostgreSQL persistence and HTTP are
implemented. Focused schema, transaction, concurrency, revalidation and real
restore acceptance have passed. The integrated API campaign preserved fourteen
complete deadline responses through restoration. Qadra provides the case-local
registry interface; its separate verification is recorded in the verification
report. The remaining operational lifecycle is pending. This decision does not assert completion of
the operational deadline workflow. See [the lifecycle](../deadline-lifecycle.md).

## Context

The profile evaluator combines an exact declared source, optional calendar,
ordered quantity and applicability answers. Its output can be blocked, contain
a civil candidate without an operational cutoff, or establish an exact instant.
Later revisions of a profile or source must not change that historical output.
The operator also needs to record attention independently of mathematical
reevaluation and of expiration derived from the current clock.

Some source receipts bind canonical values but not every readable historical
projection. Capturing only those receipts would leave administrative metadata,
hearing attendee names and document support names outside the deadline's own
commitment. Conversely, an ordinary active administrative update to the case
must not silently change the reviewed calculation or create an artificial
conflict unrelated to its sources.

## Decision

Use a stable deadline identity within an immutable case. Register and Correct
resolve an exact profile that must equal its currently published head, the
explicit historical source and calendar selections, and their separate observed
heads. The responsible account must be active staff with current case access. Assignment does not grant that access. Only Owner and an
assigned Litigator manage deadlines; assigned Paralegal reads; Client is denied.

Store the evaluation inputs as DEVI1 and the historical result as DRES1. DEVI1
preserves unknown declarations, omitted quantities, precision, offsets and
condition order. DRES1 preserves extraction outcome, arithmetic operands,
intermediate traces, calendar classifications, blocks and any due instant.
Decoding validates representation and internal consistency without invoking
the current arithmetic evaluator. Exact references and the profile algorithm
remain part of the enclosing calculation evidence.

Keep registry status, attention and clock-based expiration separate. Attention
is pending or an explicit recorded declaration with its own time precision,
statement and locator. SetAttention can correct that declaration or explicitly
return it to pending, always with a reason and another immutable revision.
SetAttention and terminal Retire preserve the previous calculation, sources,
responsible snapshot and administrative capture. Correct explicitly reevaluates
inputs and preserves attention. None of these actions certifies filing validity.

Use separate reviewed and captured commitments:

- DLRV1 binds title, selected profile and inputs, responsible snapshot, complete
  historical source evidence, stored result, attention and registry status.
- DLST1 additionally binds the exact observed case administration. Historical
  administration inside a source is included in both commitments.
- DLTX1 binds actor, case, deadline, operation, action, expected revision,
  reviewed-state digest and optional reason. It is the submitted confirmation.

Reproducing the operation inside the shared audited transaction must preserve
DLTX1. Register and Correct may capture a later active case administration; a
closed case, regression of administrative revision, or rewrite at the same
revision is rejected. SetAttention and Retire preserve DLST1 as prepared.
The final revision, both state digests, operation receipt, typed dependencies
and an audit event identifying the submission and capture commit together.
The hashes detect inconsistent representations; they are not external signatures
or a replacement for authorization, immutable storage and audit verification.

Preparation does not reserve an operation or a timestamp. Commit rechecks actor,
case access, responsible eligibility where needed, terminal status, expected
revision and exact dependencies before writing. History responses expose lightweight receipt headers. The adapter verifies each
against its full historical detail before returning authorized, paginated
results; an exact-revision query supplies that complete capture.

The five `0017_deadline_*.sql` migrations add `case_deadlines` and
`case_deadline_revisions`. A deferred foreign key requires the first complete
revision; subsequent revisions are consecutive and retirement is terminal.
Typed foreign keys preserve selected and captured-head references, including the
separate parent revisions of a notification. The original R0 administration is
represented by a null revision and its CADM1 bytes, reconstructed from the
original case metadata. A recorded administration resolves its exact revision.
No new CADM1 decoder or fabricated administration history is introduced.

Persist both review/capture bytes and their hashes, and strictly project DLTX1
in SQL. DEVI1 is fully parsed before projecting dependencies. DRES1 remains a
bounded Rust decoding responsibility; SQL does not duplicate arithmetic. Nullable
`due_at_seconds` and `due_at_nanoseconds` must be present together and match the
captured result. Attention JSON admits only the declared temporal components,
including explicit unknown precision and an absent offset.

Startup verifies the installed catalog and append-only runtime privileges,
then checks every captured revision in bounded pages. Inventory reads never
substitute current heads or rerun arithmetic. Canonical size limits, their
conservative derivation and operational recovery instructions are recorded in
[the deadline record contract](../deadline-records.md) and
[database operations](../database-operations.md).

An initial legacy import rejects either deadline table being occupied even when
no audit event accompanies a partially restored row. Existing import receipts
remain reconcilable without deleting later deadline history. Backups must include
both tables, all historical dependencies, captured identities, audit and the
source-event sequence; equal row counts alone do not prove restoration.

The focused restore acceptance compares complete rows and canonical bytes,
audit, sequence state and literal catalog expressions before and after
`pg_dump`/`pg_restore`. It retains history after source/profile retirement and
actor/responsible revocation, then verifies exact reads and new attention by a
different Owner. The integrated HTTP campaign separately covers days, months,
hours, four roles, isolation, conflicts, attention and retirement, and compares
fourteen complete restored deadline responses. These are backend and transport
results; they neither establish browser acceptance nor certify the legal
applicability of synthetic profiles. Workspace-wide verification is reported
separately in [the verification report](../verification-report.md).

### Case-local interface and eligible responsible selection

Expose an audited, paginated selection of eligible accounts under the case's
deadline collection. Return only ID, email and role: active Owners have global
case access, while active Litigators and Paralegals require membership. Filter
before pagination and do not duplicate an Owner who is also a member. Reading
candidates neither creates membership nor reserves eligibility; Register and
Correct still revalidate it under the audited commit. Historical attention and
retirement preserve the previously captured responsible even after revocation.

The browser presents captured inputs, result, receipts and exact profile/source
revisions without implementing a second evaluator. Selecting a source revision
does not substitute its current head. Unknown declarations require a reason;
absent ordered quantities remain absent. A blocked preparation requires explicit
confirmation before it is recorded. Owner and assigned Litigator manage the
registry; assigned Paralegal reads and Client has no deadline navigation or data.

An uncertain submission retains its operation and expected revision. Reconcile
through an exact-revision query and compare the full receipt and reviewed state;
never automatically resend the write. A conflicting revision requires comparison
with the current base before preserving and preparing the draft again. Session
and case changes dispose scoped requests; late responses cannot populate the new
context. Closed cases retain reads and disable mutations.

The browser displays integer seconds, nanoseconds and the declared offset without
rounding them through a millisecond-only date representation. Day-count traces
use bounded progressive display and a contained horizontal table on narrow
screens. Reference expansion remains an authorized request for the captured
revision, not a request for the latest source.

## Consequences

The backend can preserve incomplete calculations without inventing deadlines.
Recording attention does not adopt a newer rule, calendar or source. Historical
readers do not depend on current arithmetic behavior, current account names or
current administrative case status.

Canonical inputs and output need explicit bounded decoders and independent wire
vectors. New arithmetic behavior requires its own algorithm selection; replacing
stored output during a read is prohibited. The persistence adapter must resolve
historical references with exact revisions rather than call a current-head loader.

This implementation does not implement durable reevaluation, alert delivery,
email idempotency, the joint agenda or the remaining Qadra screens. Those remain
required by [the product scope](../product-completion.md). The profile's declared
applicability and example corpus do not establish an authoritative legal opinion.
