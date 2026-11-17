# 0091 - Activity alert scans audit only changes

## Status

Accepted. This record refines the scanner part of
[ADR 0039](0039-durable-activity-alerts.md); every other part of ADR 0039
remains in force.

## Context

ADR 0039 has hearing and deadline writers mark their alert state dirty inside
their audited transaction, and has the scanner reconcile new subjects,
preferences and changes with durable progress and a budget. Besides dirty
subjects, the scanner walks every hearing, deadline, resource hearing and
precautionary hearing in a loop and starts a new pass one second after the
previous one ends.

The scanner appended `alert.reconciled` for every subject it visited and
`alert.scan_completed` for every finished pass, even when the reconciliation
scheduled nothing, superseded nothing and left the subject state unchanged.
With a few dozen subjects this produced about 26 events per pass and hundreds
of thousands per day, all of them recording no change. Every audited event
lengthens the history that whole-chain verification, restore capture and the
opening timestamp check read, and the scanner holds the shared audit lock while
it writes them.

No document asks for an event per pass or per unchanged subject. The audit
principle of [ADR 0016](0016-case-document-transactions.md) is that no change
is committed without its event. An idle scan over an empty inventory already
writes nothing.

## Decision

A scanner reconciliation appends `alert.reconciled` only when it changes alert
evidence: it creates or rewrites the subject state, inserts a schedule row,
supersedes one, or rewrites one with a different generation, status or
payload. A schedule row whose recomputed values are identical is left
untouched. The event, its resource text and its place in the same transaction
as the writes are unchanged.

Finishing a pass no longer appends `alert.scan_completed`. Advancing the scan
cursor, counting passes and clearing the dirty mark of a subject whose
reconciliation changed nothing are scan progress, not alert evidence; the writer
that set the mark already audited its own change.

Activation (`alert.activated`) and delivery events are unchanged.

## Consequences

An unchanged pass writes no audit event, so the history grows with alert
changes instead of with elapsed time. The cursor update of each visited subject
remains as technical progress.

The audit trail no longer shows when the scanner last visited an unchanged
subject or finished a pass. The cursor row (`alert_scan_cursor.cycle`) keeps the
pass count.

Events already written are part of the hash chain and are kept.

The pass still restarts one second after the previous one ends and still takes
the shared audit lock for each subject. With the lookup indexes of
[ADR 0090](0090-audit-event-lookup-indexes.md) each subject holds it briefly;
the interval is not changed here.
