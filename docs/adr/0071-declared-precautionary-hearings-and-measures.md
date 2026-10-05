# ADR-0071: Declared precautionary hearings and measures

## Status

Proposed.

The local implementation includes appointment and measure values, exact contexts,
source admission, instruction commitments, complete capture receipts and bounded
history validation. Review appointments resolve actual target measures; decisions
reconstruct all declared effects and exact ordinary Initial or precautionary
anchors. Existing no-anchor and appointment canonical bytes are preserved.

The authorized appointment service and its readers are implemented and verified locally.
Scheduling, replacement and cancellation retain complete origin-bound prefixes,
while explicit replay preserves the original instruction, actor and timestamp.
These application ports do not establish a working database or HTTP workflow.
The authorized decision service also admits exact support, confirms both digests,
and validates the complete original group and ancestor closure on replay and commit.
Decision readers verify full immutable groups, bounded pagination and shared
source/ownership consistency with current staff authorization. A first pure
administrative capture corrects one exact judicial measure using its complete
group ancestry, retaining the last actual judicial declaration and support.
The mixed record resolver also validates repeated corrections with complete
judicial and administrative owners. A pure mark-only operation appends an
entered-in-error capture while retaining the recorded terms and judicial evidence.
Judicial decisions and fresh Review appointments do not yet consume administrative
records. Optional administrative replacement, current mutation eligibility,
authorized correction services, durable storage, HTTP, Agenda, alerts, Qadra and
restoration remain pending. The bounded contract and source
catalog are in [the scope document](../precautionary-hearings-scope.md). Executed
checks and their limits belong in [the verification report](../verification-report.md).

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
an entire history. Review-purpose captures require actual measure captures and
all owning-group dependencies. The old empty-evidence entry points reject Review
values; no fabricated measure source makes them admissible. Replacement,
cancellation and supplied history resolve the union of all selected revisions,
including different revisions of one identity. Each individual target list keeps
its original bound. Capture clocks follow exact selected measure provenance;
a newly valid capture time cannot repair an invalid predecessor's own clock.

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

The local domain correction whitelist is explicit: conditions, validity statement,
start declaration, an already-present end declaration, and the text of the existing
supervision variant. Preserve exact subject, class, end presence, supervision
variant and selected supervisor. An unchanged normalized result is rejected.
Validated temporal components, precision and optional offsets can change as
explicitly supplied; precision need not increase, and no missing component is
invented. Every judicial decision field remains immutable. These checks cannot
establish faithful transcription of its support.

`MCVAL1` binds the correction conditions, length-framed original-format `MVAL1`
and supervision text. Domain values alone create no revision or judicial effect.
Administrative records share the measure revision sequence while retaining a
distinct receipt/owner family. Exact historical references retain their original
meanings.

### First administrative correction capture

The local pure capture accepts one `Correct` command targeting an exact judicial
M and its complete original G ancestry. It derives the next measure revision,
retains identity, judicial origin, original record root and last judicial action,
and applies only the domain whitelist above. Terminal judicial declarations can
be corrected without changing their terminal action. Every D field and earlier
M/G capture remains unchanged; no new judicial decision or group is fabricated.

The retained last-judicial reference identifies the actual selected M and its
owning G. Support comes from that G's decision, including its complete metadata.
After a modification this is the modification support, not a substitution of
initial imposition support. Subject and supervisor sources remain exact, and
their projection is rederived from those same revisions.

The four administrative commitments have separate responsibilities:

- `MATXN1`: recording actor ID/email/role, case, administrative operation, exact
  target, expected context, reason and action. `Correct` uses tag 0 followed by
  its `MCVAL1` values; `MarkEnteredInError` uses tag 1 without a values payload.
- `MAPR1`: complete instruction and digest, complete `PCTX1`, retained support,
  resulting revision, roots, exact last judicial owner/reference, action,
  capture validity, complete values, sources and projection.
- `MARCR1`: one complete resulting row, case/operation, recording actor, context,
  support, review digest and UTC capture clock. It excludes its own digest and
  the owning administrative receipt digest.
- `MAGR1`: complete review and digest, complete owned row and digest, and capture
  clock. Its own digest is excluded. Original judicial and appointment formats
  retain their existing bytes.

The checked receipt owns exactly one row. Full reconstruction rejects missing,
extra or altered rows and provenance even after outer hashes are recomputed.
The administrative operation cannot reuse a judicial operation in the supplied
closure. Before hashing history, bound the combined owner count at 256 and rows
at 8192, including the new administrative owner and row. Compare the new context
with every supplied immutable source and advance it from the selected judicial
context. Capture time preserves nanoseconds, requires supported UTC, and cannot
precede the selected capture or new context provenance.

This is historical consistency checking. It does not freshly admit encrypted
support, authenticate current access, prove durable origin/current head, or show
the absence of dependants. The mixed resolver and mark-only extension below
reuse these receipt shapes. Authorized administrative workflow and persistence
remain separate work.

### Mixed record history and repeated correction

The mixed resolver accepts exact judicial or administrative measure references,
with up to 32 selected identities and a complete flat owner inventory. It keeps
the same measure ID/revision/digest selector and preserves each historical
meaning. A private checked projection exposes the selected record's values,
sources, context and capture time separately from the actual last judicial M/G,
its action, origin and support. It never fabricates a judicial M to represent a
correction.

Combined operation UUIDs and measure ID/revision ownership must be unique across
G and administrative receipts. Check nested shapes and the combined limits of
256 owners and 8192 rows before hashing or cloning; a new correction reserves
its owner and row. Reject missing, extra, foreign, cyclic or contradictory
evidence and reconstruct every unselected sibling of an owning group too.

Administrative dependencies are discovered iteratively. The existing judicial
closure is validated once, followed by one reconstruction of each administrative
owner in predecessor order. A shared inventory compares immutable sources across
all owners. A repeated correction advances from the selected correction's exact
revision, context and time, while retaining the original root and actual last
judicial declaration/support. Its complete one-row receipt remains subject to
the same reconstruction and provenance checks.

The judicial-only entry points remain wrappers over borrowed history; existing
`MATXN1`, `MAPR1`, `MARCR1`, `MAGR1` and judicial bytes are unchanged. Current
heads and absence of dependants still require an authorized store. This resolver
does not yet extend judicial G predecessors or fresh Review consumption to
administrative records, implement joint replacement, or establish persistence.

### Entered-in-error capture validity

The pure `MarkEnteredInError` action declares that an exact captured record was
entered in error. It accepts a Valid judicial or administrative predecessor and
appends the next revision of the same measure identity. Its one owned row retains
complete values, sources, projection, record root, judicial origin, actual last
M/G, judicial action and support. Only capture validity becomes EnteredInError;
the new operation also records its actor, context, reason and capture time.
Terminal judicial actions remain terminal. This action does not revoke, cease,
substitute or annul a judicial decision and creates no replacement identity.

`prepare_measure_administrative_record_with_history` accepts Correct and
MarkEnteredInError. The existing `prepare_measure_record_correction` and
`prepare_measure_record_correction_with_history` entry points remain Correct-only
and reject Mark. Both actions reject an EnteredInError predecessor; there is no
reactivation through correction or another mark. Preparation retains the same
context, source, ownership, revision and supported UTC clock checks, including
the selected administrative record's context and time.

MATXN1/MAPR1/MARCR1/MAGR1 retain their existing layouts and Correct bytes. The
mark uses its distinct instruction action tag and the existing capture-validity
discriminator. Matchers, origins and mixed historical resolution reconstruct the
complete marked receipt; an EnteredInError record is readable historical evidence.
Its existence does not rewrite older Valid captures or their exact references.

The supplied closure proves neither a current head nor absence of later
dependants. Storage admission must establish both under the mutation lock, with
current authorization and atomic audit. Those checks and SQL remain pending.
Fresh Review and judicial consumers of administrative records, and the optional
atomic replacement with a fresh identity and explicit administrative link, remain
pending. Their future admission must distinguish capture validity from retained
judicial status without invalidating earlier historical receipts.

### Decision anchors and shared dependency evidence

Anchor references discriminate absence, an ordinary Initial revision and a
precautionary revision with tags 0, 1 and 2. `MDTXN1` binds the exact family, ID,
revision and family commitments. Full `MDPR1` and `MDCR1` material uses `MHIA1`
for ordinary Initial detail and the unchanged `PHCR1` plus its capture digest
for precautionary detail. `MHIA1` retains the complete ordinary snapshot,
receipt, scheduling/recorded context, actor, UTC capture clock, participant
projections including typed kind and subject reference, and optional support.
It does not claim full participant source snapshots absent from ordinary detail.

Historical scheduled and cancelled revisions remain valid exact anchors; neither
proves that a hearing occurred or produced a decision. The decision capture
cannot predate its anchor. Known context references and immutable source copies
must agree throughout the supplied evidence. Anchor targets and affected measures
need not coincide: each selection has its own exact meaning.

Owning-group traversal follows effect predecessors and precautionary Review
anchor targets together. It checks the bounded graph iteratively and reconstructs
parents before dependants. Flat anchor validation receives only checked parent
material; it does not recursively restart public history validation. The exact
closure rejects missing, unrelated and cyclic groups. Existing no-anchor bytes
and appointment formats retain their prior meanings and vectors.

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

### Authorized appointment preparation and confirmation

The scheduling service authenticates the full current principal before lookup.
Owner and Litigator may prepare and submit. A Ready result retains the exact
observed context, encrypted document, selected participant captures, complete
previous appointment prefix and actual measure dependency closure. Cancellation
retains previous sources; it does not re-admit historical support. Scheduling
and replacement admit their exact encrypted support outside the audit lock.
Only the service constructs the private prepared value. Supplied historical
validity cannot substitute for current membership, case activity or source heads.

Confirmation requires both instruction and full-review digests. This includes
changed source provenance and cancellation's newly observed context, even when
the instruction bytes remain unchanged. Prior appointment captures and measure
anchor captures share a full immutable-source inventory before preparation
succeeds. Equal review bytes cannot excuse conflicting capture provenance.
The complete prefix, including the new capture, is bounded to 256 revisions.
Evidence over a bound is rejected, never silently truncated.

Reauthenticate and check supported UTC observations before commit and before
disclosure. The fresh capture cannot precede the checked sources or the last
precommit clock observation. An exact raced replay retains its earlier original
capture. Replay requires the original recording account ID and complete command;
its historical email and role remain unchanged after an authorized profile edit.
Returning a result requires the same full current principal as request admission.

Under the shared audit lock the store must recheck the complete principal,
membership, active context, current predecessor, source heads, case/document
association and all reviewed material. It atomically writes capture, head,
origin, operation and audit, or returns the original exact raced operation.
Readers require current staff access, including Paralegal, and deny Client.
Closed cases remain readable; read audits and durable-origin checks belong to
the store. Exact revision and operation reads return their original complete
prefix. Lists use at most 20 current heads in strict UUID order, with an exclusive
cursor and validated continuation. Application readers verify every full receipt
and shared immutable source, without re-admitting its historical support.

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
