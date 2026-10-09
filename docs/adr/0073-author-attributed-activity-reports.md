# Author-attributed activity reports

## Status

Implemented with focused and native acceptance. Integration requires the
closing checks of the published revision.

## Context

The state report described in `docs/adr/0060-durable-authorized-case-reports.md`
filters case creation and derives workload from current assignments. These data
cannot establish who uploaded documents, recorded procedural activities or
marked deadlines attended during a period. Changing an assignment must not
transfer those contributions to another account.

Procedural revision histories retain the recording account UUID and operation
instant. Older document uploads retain an audit email, which is not sufficient
to infer a stable account identity retrospectively.

## Decision

Reuse the durable report request, authorization, queue, encrypted capture,
rendering, notice and download workflow. Introduce a separate activity kind and
UTC operation interval. Keep the legacy HTTP shape, stored representation and
canonical state-report bytes unchanged. New activity captures use storage
version 2 and separate canonical request/snapshot domains.

Persist an original document upload origin in the existing document/audit
transaction. It contains the document and case, author UUID and captured email,
operation instant and exact upload audit sequence. Appending a content version
or changing classification does not create another original upload. Do not
backfill UUIDs from historical audit emails.

The capture holds exactly three counts per case and author: original documents
uploaded, original procedural activities recorded and deadlines marked attended.
Aggregate totals are derived for PDF and CSV from that same immutable capture.
Use operation time, not the date declared in a judicial record or deadline
attention. A pending-to-recorded attention transition contributes once per
deadline and author within the requested period; editing its description adds no count.
Reads, preparations, corrections and exact operation recovery add no activity.
Separate confirmed uploads with distinct document identities remain separate;
this report does not redefine document upload idempotency.

Author selection keeps the existing active Litigator account eligibility.
For activity it additionally admits authors of captured-source operations in
cases the requester can currently access, even after the author's assignment
ends. This does not reconstruct the account's role at operation time. Owner
can report office cases; Litigator can report only currently assigned cases.
Revalidate all captured case access on subsequent report operations.

A capture is conservatively marked document-incomplete when any included case
has a document series without an original upload origin, even if its upload
may predate the requested period. The PDF states that counts cover identified
uploads; CSV carries `documents_complete` on every row. Missing authorship is
not reported as a complete zero. No ranking or legal-success metric is added.

The fixed procedural-activity catalogue includes original resolution and
notification records; original resource acts of interposition, admission,
inadmissibility, withdrawal and resolution; original declared hearing
sessions/results; and original judicial precautionary decisions. Count each
original record identity once. Agreements within a hearing session/result and
measures within a precautionary decision do not contribute separate activities.
Scheduling a hearing is not a session/result record. Corrections and exact
retries retain the original activity identity instead of adding counts.

The aggregate includes hearing-session/result and judicial precautionary-decision
sources. Focused source checks and a real PDF/CSV catalogue journey establish
their recorded-activity semantics; final delivery gates remain separate.
The remaining delivery criteria are recorded in `docs/technical-closure.md`.

## Consequences

One report request retains one captured result and a common digest for both
formats. The existing state report remains available with its original meaning.
Both modes use the existing bounded case, actor, capture and artifact budgets;
exceeding them fails instead of silently omitting rows.

Historical document attribution remains explicitly incomplete. Changes to
current author account eligibility may change a later report's population;
they do not rewrite a previously captured report. Operational email delivery,
identity administration and historical role reconstruction remain separate.
