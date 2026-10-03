# ADR-0069: Declared scheduling for resource hearings

## Status

Accepted architecture; implementation remains in progress. The local domain
values and application preparation service exist and have focused checks with
controlled ports. There is no resource-hearing persistence adapter, HTTP route,
idempotent creation, agenda projection, alert integration or user interface yet.
These remain subsequent work in the same delivery. Neither global CI nor a new
deployment is asserted here. The current contract is in
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
`HEAR1`.

Scheduling requires a declared instant with its original offset and a basis
statement with an exact document-version reference and digest. The application
accepts that support only when it was already admitted in the selected resource
or selected act. It does not upload, re-admit or infer a judicial determination
from that content. Participant selection is bounded to 32, unique by identity,
and normalized by UUID; each selection retains its revision.

`ResourceHearingStore::prepare` must authorize the current case before loading
material, resolve historical resource/act captures independently from the current
head, and return current selected participant revisions. This is a read-only
contract, not an implemented database authorization boundary. The application
checks receipts, exact references, current resource revision and active status,
administrative coherence, compatible classifications, support and participant
projections. It rejects archived participants.

`ResourceHearingService` admits Owner or Litigator before calling the port.
After preparation it authenticates again and requires equality of the complete
`Principal`, including its captured email. A valid role alone cannot justify
returning a review prepared for another principal or changed identity.

Use domain-separated representations. `RHEAR1` commits normalized scheduling
values without a stage or receipt. `RHPR1` hashes the prepared review, including
command identities, selected captures, observed resource head, actor,
administration, support and participant projections. The exposed
`submission_digest` is review evidence only; it is not a durable operation receipt
and does not reserve an identifier, resource revision or appointment slot.

## Consequences

The caller can review declared scheduling against exact existing evidence without
fabricating ordinary procedural progress. The preparation is deterministic for
the same validated material, while changes to reviewed data alter its digest.
Missing or unknown resource mode cannot be inferred from hearing classification.

The future write path must define and implement durable authorization,
revalidation, atomic creation with its association, idempotent reconciliation,
history and queries. No successful preparation proves those properties. Agenda,
alerts and Qadra must compose that completed path, not manufacture an ordinary
hearing from this draft. Those pending implementations require their own failing
tests and evidence; the present controlled-port checks cannot substitute for them.
