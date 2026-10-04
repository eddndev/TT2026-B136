# ADR-0069: Declared scheduling for resource hearings

## Status

Accepted architecture; implementation remains local, unmerged and undeployed.
Domain values, application preparation/submission, the PostgreSQL adapter,
initial typed association, durable origin and strict schema/inventory exist.
Generic association DTOs support this family. Dedicated historical queries and
their scheduling/read router are implemented locally, including server
composition. The combined agenda and its Qadra historical panel are implemented
locally. Qadra association queries also recognize the separate hearing family;
scheduling forms and explicit recovery are implemented locally. Alerts and
integrated acceptance remain pending.
Focused schema, application, PostgreSQL adapter and generic HTTP DTO checks
passed, as did the new application and PostgreSQL read checks, the specific
stored-integrity error for incomplete creation evidence, dedicated HTTP routes
and server composition.
Neither integrated acceptance nor global CI is asserted here.
The current contract is in
[resource-hearings.md](../resource-hearings.md).

## Context

Ordinary hearings bind their scheduling values to ordinary procedural stages.
A hearing concerning an appeal or written revocation cannot require an invented
stage merely to reuse that workflow. Existing resource-activity associations
preserve historical references but do not, by themselves, create an appointment.
See [ADR-0041](0041-exact-resource-activity-associations.md).

The closed classification distinguishes appeal arguments and a hearing called
for a complex written revocation. It does not turn an oral revocation into a
separate automatic appointment or decide when the court must schedule a hearing.
The official source and its narrow scope are recorded in the contract.

A resource can have a selected historical capture and a newer current head.
Replacing the selection silently would alter its evidence. Conversely, accepting
only a historical resource would omit the current revision and archive checks.
Participant directory references and an uploaded document also do not establish
judicial authority or legal attendance requirements.

## Decision

Use a separate `resource_hearings` domain family with exactly
`AppealArguments` and `WrittenRevocation`. Require an explicit compatible written
resource mode. Reuse the bounded time, modality, venue, note, participant and
support values, but do not change ordinary `HearingKind`, its stage policy or
`HEAR1`. Give resource hearings their own hearing identity, operation identity and
positive revision type. A creation begins at revision one; these types do not
supply a replacement or cancellation workflow by themselves.

Scheduling requires a declared instant with its original offset and a basis
statement with an exact document-version reference and digest. The application
accepts that support only when it was already admitted in the selected resource
or selected act. It does not upload, re-admit or infer a judicial determination
from that content. Participant selection is bounded to 32, unique by identity,
and normalized by UUID; each selection retains its revision.

`ResourceHearingStore::prepare` must authorize the current case before lookup.
It returns either exact prior creation evidence with its origin marker, or
material for a new preparation. The latter resolves historical resource/act
captures independently from the current head and returns current selected
participant revisions. The application checks receipts, exact references,
resource revision and active status, administrative coherence, compatible
classifications, support and participant projections. It rejects archived
participants. The PostgreSQL adapter enforces current case authorization and
current participant selection under the shared audit lock. Preparation changes
no business data but commits its authorized read audit before returning.

Construct `PreparedResourceHearing` only through validated preparation. It keeps
the full material and principal as well as the review. Submission requires the
exact reviewed digest, reauthenticates the complete current `Principal` before
commit, validates the returned creation against the complete review, and
reauthenticates before returning evidence. Only Owner or Litigator may enter
this flow. A changed principal cannot receive the prior result merely because
its role remains allowed.

Require the store commit to reauthorize and revalidate all reviewed material
under the shared audit lock, then write the hearing, initial resource association,
origin marker and audit in one transaction. An exact raced operation must return
the original creation and timestamp; a conflict must write nothing. The adapter
implements this transaction. Its database tests are reported separately from
controlled application checks. Converting a prepared value into creation
evidence does not itself persist anything.

Use domain-separated representations. `RHEAR1` commits normalized scheduling
values without a stage or receipt. `RHPR1` hashes the reviewed command identities,
selected captures, observed head, actor, administration, support and participant
projections. `RHCR1` binds that review digest, initial revision, recording time and
additional participant provenance, sorted and explicitly bound to each
participant identity and revision. The creation includes the full historical
material, the initial real association and a matching origin marker. Construct
the association with `prepare_activity_change`, using the same actor, context,
timestamp and operation UUID under its own type. Add `ResourceHearing` as tag 2
in `RASL1`; preserve the existing tags and bytes. `RHPR1` and `RHCR1` remain
unchanged and expose their canonical bytes to storage validation. Keep the
hearing detail independent of its association: validate the detail first, then
the complete creation, avoiding a recursive receipt dependency.
Neither credential provenance nor a
successful capture check certifies legal authority or grants current access.

Recover an uncertain response only from the original immutable creation and its
origin, never from the mere presence of an independently created hearing or
association. Replay requires the exact command, scope and original actor identity;
submission also requires the reviewed digest. It preserves original evidence and
timestamp without another commit. A historical author email is not overwritten
with the current email. Current authorization and equality of the current full
principal before and after the call remain required. Removing a current
association must not remove origin evidence or enable a duplicate creation.

Separate `ResourceHearingReadStore` and `ResourceHearingReadWorkflow` from the
write workflow. Share the existing PostgreSQL adapter; list and get verify the
complete original creation, including its initial association and durable
origin. Authorize current account and membership before lookup under the audit
lock, commit the read audit, then reauthenticate the full principal in the
application. Owner reads all cases, assigned Litigator and Paralegal read their
cases, and Client is denied. Closure, resource archive and later unlinking do
not erase historical access for an otherwise authorized reader.

Bound lists to 1..20 creations, default 10, in ascending UUID order with an
exclusive cursor. Do not filter by current association state or promise
chronological order. Validate the complete page scope, order and continuation,
each creation receipt, and a non-regressing UTC read clock. Return historical
creations without a field suggesting current operational validation.

Expose prepare/submit and list/get/exact routes in the resource-hearing family,
binding URL parents to commands and responses. Preserve the existing separate
DTO; do not transform it into an ordinary hearing. Reuse explicit submission
for uncertain outcomes, as with contextual deadlines. There is no extra
reconcile endpoint: a repeated exact submit can create when the first attempt
did not commit, so callers must request it explicitly. GET never retries writes.

Migration family `0030_` stores hearing roots and captures, extends exact
association references and updates their guards. Runtime grants permit only
reading and column-bounded insertion. Startup checks the strict catalog and
replays bounded inventory without writes or repairs. It checks both capture to
origin and origin to capture, including the original association and audited
`rhl1` marker; an orphan on either side rejects startup after restore.

Propagate an uncertain commit result without automatic retry. The caller may
explicitly reconcile the same operation. A post-commit authentication failure
can withhold the response without proving that the write did not happen.

## Consequences

The caller can review declared scheduling against exact existing evidence and
confirm that same review without fabricating ordinary procedural progress.
Changes to reviewed data alter the digest. Missing or unknown resource mode
cannot be inferred from hearing classification. Support remains restricted to
already admitted evidence in the selected resource or act; this path does not
admit an arbitrary new summons.

The PostgreSQL adapter now preserves capture, original association and audited
origin atomically. Controlled-port checks remain distinct from database evidence.
Dedicated hearing queries, scheduling HTTP and server composition have local
implementation; integrated acceptance remains pending. The combined agenda adds
an independent rank after ordinary hearings and deadlines, retaining its existing
cursor version and ranks. It verifies the original association and audited origin
inside the authorized read transaction. Unlink, resource archive and case closure
do not cancel the captured appointment. Its summary exposes no invented stage or
status. Qadra opens a historical panel inside the combined agenda after reading
current case administration and the exact creation. Bind the response to every
selected summary field, including original association and capture digest. Keep
filters, accumulated rows and continuation when closing that panel; invalidate
pending reads when the view or query changes. The panel describes original
association evidence without claiming its current link state. Scheduling forms
retain raw drafts and full-principal ownership, reauthorize before prepare and
submit, require explicit acknowledgement, and preserve the exact command for
uncertain outcomes. Recovery reads never post automatically. A missing exact
creation remains uncertain; only explicit retry may resubmit that same command.
The independent creation list retains history after unlinking. Alerts and full
acceptance remain pending in this delivery; do not manufacture an ordinary
hearing from the draft. Generic association endpoints
continue to expose the separate `resource_hearing` record with exact evidence.
This local implementation does not close deployment or CI.
