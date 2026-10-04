# ADR-0071: Declared precautionary hearings and measures

## Status

Proposed.

Appointment values and their own canonical representation are implemented
locally with focused domain verification. This first checkpoint does not
establish the complete workflow or its acceptance; remote gates and integration
remain pending.
The local value extension comprises `MeasureKind`, `MeasureTime` and
`MeasureValidity` with `MVAL1`; ten focused tests and target Clippy pass after
the initial failing tests. Full workflow verification and integration remain pending.
Exact historical context validation (`PCTX1`) and instruction framing (`PHTXN1`)
are implemented locally. Their focused tests pass; these values are not capture
receipts or a complete scheduling service. Exact participant resolution also
validates selected manual or typed historical values and bound subjects; it does
not establish current access, current heads or documentary admission. A separate
local direct-support adapter delegates exact selected-document admission to the
bounded document processor; the same exact-source admission also accepts the
resolution selected by `MDVAL1`. Both use one internal integrity/format boundary;
persisted case association remains a store check.
Complete declared measure terms (`MEAS1`) and factual decision declarations
(`MDVAL1`) are also implemented locally, with a distinct immutable decision ID.
The bounded `MEFX1` outcome now normalizes declared effects and rejects duplicate
or overlapping affected identities. It does not resolve predecessor captures or
establish hearing anchors, group origins or committed effects. Exact measure
source resolution now checks the declared subject and optional supervisor,
retaining full captures separately from derived labels; it grants no current
permission, official appointment or documentary admission.
Flat imposition capture receipts and adjacent-transition checks are implemented
locally. Exact origin and supplied-chain validation are implemented too;
durable existence/current head and Review-purpose measure evidence remain pending.
Standalone initial impositions and explicit no-change decisions now build real
review, decision, measure and group captures (`MDPR1`, `MDCR1`, `MMCR1`, `MDGR1`),
from the actor-bound `MDTXN1` instruction. Full reconstruction verifies every
member, immutable source and clock while retaining declared time precision.
The history-aware extension now resolves complete owning-group ancestry and
reconstructs confirmation, modification, revocation, cessation and joint
substitution. It rejects missing, cyclic or contradictory evidence and bounds
validation to 256 groups and 8192 member rows, including a candidate. Empty-history
wrappers still reject predecessor effects. Hearing anchors remain pending;
these checks prove neither durable creation, current heads nor current access.
Precautionary workflow services, persistence, HTTP, Agenda, alerts,
Qadra and restoration remain pending. The bounded contract and source catalog
are in [the scope document](../precautionary-hearings-scope.md). Executed evidence
belongs in [the verification report](../verification-report.md).

## Context

The [ordinary hearing catalog](../../crates/domain/src/hearings/catalog.rs)
assigns one required procedural stage to each of its four kinds. Its `HEAR1`
values and existing storage constraints encode that policy. A precautionary
appointment cannot be added by selecting a convenient stage or reinterpreting
the sentencing support. The separate
[resource hearing family](0069-resource-hearing-scheduling.md) demonstrates the
necessary separation of identities and provenance.

The [CNPP](https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf), particularly
articles 153-164 and 307, distinguishes requests, hearings and judicial
decisions concerning measures. Article 159 identifies justification, application
guidelines and validity as resolution content. Article 162's review-hearing
period depends on its stated premise; it does not establish a default timer for
every review entry. Articles 347, 401 and 405 require retaining the distinction
between a case stage and a declared decision affecting measures.

Recording an appointment supplies neither a judicial decision nor proof of
compliance. The existing directory's supervisor profile also grants no judicial
authority or application privilege. Exact supporting evidence and declarations
must remain distinguishable from legal conclusions the prototype cannot make.

## Decision

### Separate appointment, decision and measure identities

Introduce a precautionary appointment family with a closed declared purpose:
`imposition` or `review`. Reuse validated time, modality, venue, note, participant
and document-support values. Preserve `HearingKind`, its stage policy and existing
`HEAR1`, `HTXN1`, `HRES1`, `RHEAR1`, `HRDL1` and `HRDC1` meanings and vectors.

The first domain checkpoint defines typed appointment, operation and positive
revision identifiers; an exact `HearingTime` preserving offset and seconds;
modality, venue and optional note; up to 32 exact participant identity/revision
references, unique and normalized by UUID; and required `HearingNote`
statement and locator with an exact `HearingSupportRef`. Review targets bind
measure identity, positive revision and digest, are unique and sorted by UUID,
and number 1..32 for `review`. `imposition` prohibits targets. An independent
`PHEAR1` representation binds all those appointment values and target fields.
Context, authorization and receipts are outside this first value encoding.

The full workflow must add exact administrative and observed stage context with
a declared basis. It may capture a communicated appointment in Investigation,
Intermedia or Trial without determining jurisdiction or legal admissibility from
the stage. Stale reviewed context, contradictory references and foreign-case
sources reject confirmation. Replacing or cancelling an appointment preserves
its previous captures and requires an expected revision and explicit reason.

### Context and instruction commitments

The context retains the complete observed administration, the exact stage entry,
and the separate historical administration cited by that stage. A later
administrative revision must not substitute for the stage's original reference.
Validate scopes, value digests, revision relationships, supported stage shape,
exact ordered documentary support and captured provenance. Initial registration
must retain the same author and time as administrative revision one. Changed
stages retain their declared values, prior stage, support metadata and capture
facts. UTC capture time and local declared time remain separate.

`PCTX1` commits this complete context. Structural historical validation permits
a later closed observed administration; it grants no authority to mutate a
closed case. The administration under which the stage was recorded must be
active and complete. Current authorization, current-head comparisons and fresh
source admission remain separate checks of the eventual transaction workflow.

`PHTXN1` binds the actor's ID, email and role, case, operation and appointment
identities, action, expected appointment revision and prior capture digest,
expected context revisions and digest, complete normalized appointment values
and explicit replacement or cancellation reason. Scheduling has no predecessor;
replacement and cancellation require its positive revision and exact digest.
Cancellation retains the previous values. A byte commitment does not establish
that the predecessor exists or that an operation was committed; the eventual
capture receipt and store must verify those relationships.

Historical framing must use the original actor role without granting that role
current permission. Never reconstruct an actor as an Owner to make validation
succeed. Submission bytes and context bytes are separate from persisted capture
receipts, current authorization and uncertain-response recovery.

### Flat appointment captures and immutable sources

`PHPR1` binds the full instruction and its digest, resulting revision/status,
both scheduling and observed contexts, complete participant and subject sources,
exact support metadata and derived participant projections. `PHCR1` binds the
full review, its digest and the UTC capture clock; it excludes its own digest.
The checked review owns this material and rejects a capture before its sources
or checked predecessor. It is structural historical evidence, not proof of
current access, fresh documentary admission or storage.

Scheduling creates revision one. Replacement and cancellation require the exact
scheduled predecessor; cancellation is terminal and preserves its original
values, scheduling context and sources while recording the newly observed
context. Full source identities remain immutable within and between supplied
captures: case/revision for administrations and stages; case/ID/revision for
participants and subjects; document ID/version for support. Equal keys require
complete value and provenance equality, including metadata not present in the
source value digest. Actual new revisions remain distinct.

The flat verifier cannot establish predecessor existence or durable origin;
adjacent validation cannot establish full ancestry or operation uniqueness over
an entire history. Review-purpose captures reject until their actual measure
captures can be verified. No fabricated measure source makes them admissible.

### Appointment origin and supplied history

An origin identifies the exact initial scheduling capture through case,
appointment, operation, revision and instruction/review/capture digests.
Extraction validates the complete receipt and rejects replacement or cancellation
as a root. The history validator requires a nonempty ascending chain from that
initial capture, compares every origin field, verifies every receipt and adjacent
transition, rejects repeated operations anywhere in the chain, and retains one
immutable-source inventory across all supplied revisions. A participant or
support omitted from an intermediate revision cannot return with changed
same-revision material.

This validates the supplied chain, including a valid initial prefix. It cannot
detect a missing valid suffix, prove that a referenced source exists in the
repository, or prove current access. The eventual store must check its inventory,
durable origin and exact current head separately. Historical reconstruction does
not change stored captures or replace their sources with current revisions.

### Decision and measure records

A declared decision has its own identity, authority declaration, source locator,
exact admitted resolution support, declared time and justification. A measure
has its own stable identity, exact subject capture, closed classification,
declared conditions, start and validity, original decision and immutable history.
One decision may affect several measures and subjects. A measure's history is
independent of rescheduling, cancellation, administrative closure and stage.

`MEAS1` binds the exact subject ID/revision/digest, class, conditions, full
unchanged `MVAL1` and supervision. Known supervision records an exact participant
revision and statement; Unknown supervision records a reason. Neither form
certifies appointment or grants account permissions. Application validation must
resolve the exact subject and supervisor material in the same case.

`MDVAL1` binds authority text, complete declared time (including an unknown-time
reason), justification and exact support with locator. The decision identity is
separate and has no invented revision model. The later group capture binds that
identity, hearing anchor, effects and measure origins atomically. The factual
values do not include a group digest that would itself depend on those values.

An optional decision anchor must discriminate an exact existing initial hearing
and, when available, result from an exact precautionary appointment. An initial
hearing is reused without a duplicate appointment. Missing scheduling evidence
remains explicit; a standalone decision must not fabricate a hearing or result.
An ordinary concluded result does not establish a measure's imposition.

### Explicit declarations and bounded atomic groups

Use the fourteen classifications in the scope document. These classifications
do not choose a measure, evaluate proportionality, validate legal combinations,
authenticate judicial authority or certify supervision. Unclassified or unknown
material remains a document or draft, without an invented `other` measure.

Keep declared decision time, declared measure start/end and actual server capture
time separate. `MeasureTime` preserves the supplied precision, components and
optional offset. `Unknown` requires a reason; `Date`, `Minute` and `Second`
prohibit an unknown-value reason. Never substitute midnight, the capture clock,
a time zone or a duration for missing evidence.

`MeasureValidity` compares a declared start and end only under these rules:

- `Date` with `Date`: compare civil dates only when their optional offsets are
  equal, including two absent offsets. Do not convert a civil date to an instant.
- `Minute` with `Minute`, or `Second` with `Second`: compare in UTC only when
  both offsets are explicit, while preserving the original values and offsets.
- Mixed precision, a clock value without an offset, unequal optional offsets
  for civil dates, or an unknown value leave ordering unresolved. Preserve the
  declarations without filling gaps or reporting them as chronologically valid.

Reject an end before the start when the pair is comparable. An absent end is
distinct from an explicitly `Unknown` end with its reason; neither establishes
indefinite validity or termination. `MVAL1` binds the declared start and its
precision, components, optional offset and reason, the validity statement, and
both presence and complete value of the optional end. This value commitment
does not create a decision, receipt or persisted measure. Display the latest
recorded declaration rather than a finding of present legal force or compliance.

`measure_changes` requires at least one change. `no_measure_change` requires no
changes and an explicit supported statement of the observed decision. Unknown
outcomes remain incomplete. Non-occurrence is appointment history, not a judicial
decision or an ordinary hearing result created to satisfy a reference.

A decision group affects at most 32 distinct measure identities, counting old
and new identities together. This is a technical bound, not a legal limit.
Normalize by UUID and reject duplicate targets; never silently split an oversized
decision. Every affected existing measure requires its exact expected revision.

The local domain constructor accepts either a supported no-change statement or
a nonempty effect list. It sorts substitution sides by UUID and effects by their
smallest affected UUID, rejecting any repeated identity across all old/new
positions. `MEFX1` binds every effect tag, exact predecessor reference and complete
proposed `MEAS1`. Modify supplies full values; application checks must preserve
subject identity, class and origin against the resolved predecessor. Supplied
intent does not establish existence, current heads or judicial admissibility.

- `impose` creates new identities and their complete declared values.
- `confirm` appends additional decision evidence with an exact predecessor while
  preserving original origin, subject, class, conditions, start and validity.
- `modify` appends changed conditions or temporal declarations with a new source;
  a class change is a substitution, not an in-place relabeling.
- `revoke` and `cease` capture supported judicial decisions, not administrative
  archiving or the effect requested by an unresolved petition.
- `substitute` binds one or more exact predecessors and one or more proposed
  successors for the same subject. Old and new identities are disjoint, unique,
  acyclic and UUID-ordered. Preserve the joint relationship without inventing
  one-to-one pairs. All revisions and links commit together.

`correct_record` is an administrative correction with a reason and exact
predecessor, using the same support. It may correct captured text or precision
but not identity, subject, class, origin, judicial effect or the historical
decision. It updates the projection only from its exact current head; it never
rewrites later effects or previously linked revisions. An erroneous subject
identity requires `entered_in_error`, preserving history and distinguishing capture
validity from judicial status. A correct replacement uses a new identity and an
administrative link in one atomic operation, not a judicial substitution.
Already-dependent history requires explicit reconciliation, not a correction
cascade.

### Exact provenance, current authorization and replay

Owner and currently assigned Litigator may prepare and confirm. Assigned
Paralegal may read. Client is denied. Closed cases preserve currently authorized
history and reject mutations. Procedural participants never substitute for
authenticated accounts. Reauthenticate the full current principal before lookup,
confirmation, replay and disclosure; captured actor ID, email and role remain
historical.

Prepare resolves exact document, participant, subject and target captures
separately from observed current heads. Admit encrypted support outside the
shared audit lock. Under the transaction lock, compare the complete reviewed
material, active administration, current authorization and expected revisions
again. Preparation creates no appointment, decision or measure.

An operation UUID binds one immutable instruction. Identical authorized replay
returns its original group, origin, capture and timestamp; different instructions
under the same UUID conflict. Independent records found by identity cannot
prove common creation. Current access revocation denies replay.

Commit the decision, affected measure revisions, links, durable origin and one
group audit event atomically. The event binds the decision and all revisions.
Failure, stale targets and concurrent replay leave neither partial substitution
nor detached audit evidence. Creating an appointment remains a separate explicit
intent. Historical reads verify captured evidence without replacing it with
current source heads or applying current legal interpretations.

### Readers, recovery and restoration

Agenda and alerts require the own-family discriminator `precautionary_hearing`,
stable total ordering and cursors binding family, identity and revision. Detail
uses its own authorized route and receipt. A decision on an existing initial
hearing reuses its row. Measures and decision groups do not become extra
appointments. Appointment alerts retain their exact origin; conditions do not
generate inferred periodic obligations, breach findings or deadlines.

Qadra must preserve a draft only for the same identity, remove approval after
session reentry and reauthorize before confirmation. Uncertain writes require
exact receipt reconciliation before an explicit retry. Session or case changes
invalidate late responses; logout or a different identity cannot restore another
user's draft. There is no automatic resubmission.

Use forward migrations, independent bounded versioned decoders and strict
schema/runtime-grant inventories. Restore all appointment and decision captures,
measure revisions, corrections, substitution and administrative links, original
actors, receipts, origins and audit sequences. Reject orphan or ambiguous origins,
unknown versions, inconsistent scope and forged commitments. Reconstruction must
not reclassify measures, recalculate effects or substitute current source heads.
Restart and backup restoration must reproduce exact reads and replay without
duplicate revisions or audit events.

## Consequences

Separate families preserve existing canonical evidence and avoid an invented
stage rule. Distinct records support several measures from one decision and
preserve atomic substitutions, at the cost of new storage, readers and recovery
contracts. Domain values alone cannot prove transactional or access guarantees.

The complete delivery requires focused tests of the invariants above, database
failure and concurrency checks, HTTP restart/restoration acceptance, and Qadra
desktop/mobile recovery evidence. API, operations, verification and affected
academic documentation must describe the reproduced behavior. Legal deadline
qualification, legal adjudication and official supervision remain separate.
