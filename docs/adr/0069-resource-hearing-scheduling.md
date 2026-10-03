# ADR-0069: Declared scheduling for resource hearings

## Status

Accepted architecture; implementation remains in progress. Local domain values,
application preparation, exact-digest submission and replay validation exist.
Focused checks use controlled ports and an in-memory store. There is no
resource-hearing PostgreSQL adapter, HTTP route, agenda projection, alert
integration or user interface yet. Durable atomic creation and recovery remain
subsequent work in the same delivery. Neither global CI nor a new deployment is
asserted here. The current contract is in
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
participants. Current case authorization and current participant selection are
port obligations; a PostgreSQL boundary has not been implemented for this flow.

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
the original creation and timestamp; a conflict must write nothing. This is an
explicit adapter contract, not evidence of durable atomicity from application
tests. Converting a prepared value into creation evidence does not itself persist
anything.

Use domain-separated representations. `RHEAR1` commits normalized scheduling
values without a stage or receipt. `RHPR1` hashes the reviewed command identities,
selected captures, observed head, actor, administration, support and participant
projections. `RHCR1` binds that review digest, initial revision, recording time and
additional participant provenance, sorted and explicitly bound to each
participant identity and revision. The creation includes the full historical
material and a matching origin marker. Neither credential provenance nor a
successful capture check certifies legal authority or grants current access.

Recover an uncertain response only from the original immutable creation and its
origin, never from the mere presence of an independently created hearing or
association. Replay requires the exact command, scope and original actor identity;
submission also requires the reviewed digest. It preserves original evidence and
timestamp without another commit. A historical author email is not overwritten
with the current email. Current authorization and equality of the current full
principal before and after the call remain required. Removing a current
association must not remove origin evidence or enable a duplicate creation.

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

Controlled-port checks establish the application protocol and an in-memory
recovery example. They do not prove real process-restart recovery, database
rollback, locking or durability. The PostgreSQL adapter, initial association
write, immutable origin storage, history and queries still require implementation
and verification. HTTP, agenda, alerts and Qadra must compose that completed path;
they must not manufacture an ordinary hearing from the draft. These pending
parts remain within the same functional delivery.
