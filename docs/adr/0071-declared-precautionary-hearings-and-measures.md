# ADR-0071: Declared precautionary hearings and measures

## Status

Proposed.

Appointment values and their own canonical representation are implemented
locally with focused domain verification. This first checkpoint does not
establish the complete workflow or its acceptance; remote gates and integration
remain pending.
Precautionary application services, persistence, receipts, HTTP, Agenda, alerts,
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

A declared decision has its own identity, authority declaration, source locator,
exact admitted resolution support, declared time and justification. A measure
has its own stable identity, exact subject capture, closed classification,
declared conditions, start and validity, original decision and immutable history.
One decision may affect several measures and subjects. A measure's history is
independent of rescheduling, cancellation, administrative closure and stage.

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
time separate. Retain civil-date and instant precision, explicit offsets and
reasons for unknown values. Reject contradictory comparable bounds; do not
invent midnight, a duration, time zone or termination. Display the latest
recorded declaration rather than a finding of present legal force or compliance.

`measure_changes` requires at least one change. `no_measure_change` requires no
changes and an explicit supported statement of the observed decision. Unknown
outcomes remain incomplete. Non-occurrence is appointment history, not a judicial
decision or an ordinary hearing result created to satisfy a reference.

A decision group affects at most 32 distinct measure identities, counting old
and new identities together. This is a technical bound, not a legal limit.
Normalize by UUID and reject duplicate targets; never silently split an oversized
decision. Every affected existing measure requires its exact expected revision.

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
