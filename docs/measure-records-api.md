# Administrative measure operations and historical reads

These routes expose declared administrative changes and actual persisted measure
records. They are implemented in the precautionary delivery branch; focused HTTP
verification and composed-server acceptance remain in progress. They do not yet
establish delivery of the complete Agenda, alerts or Qadra workflow.

All paths below start with `/api/v1/cases/{case_id}`. Requests require the current
bearer identity. The services recheck case access: Owner and assigned Litigator
may write; assigned Paralegal may also read; Client is denied. Every response is
`Cache-Control: no-store`. Exact reads and original-operation recovery remain
available on closed cases when the current principal is authorized.

| Method | Suffix | Result |
| --- | --- | --- |
| POST | `/measure-administrative-operations/prepare` | Reviewed normalized command |
| POST | `/measure-administrative-operations/submit` | Original atomic capture |
| GET | `/measure-administrative-operations` | Ordered operation page |
| GET | `/measure-administrative-operations/{operation_id}` | Original operation |
| GET | `/measures` | Current measure record page |
| GET | `/measures/{measure_id}` | Current actual record |
| GET | `/measures/{measure_id}/revisions/{revision}?capture_digest={digest}` | Exact historical record |

## Commands and confirmation

Prepare accepts an object with exactly `case_id`, `operation_id`, `target`,
`context`, `reason` and `action`. The case must match the path. `target` is
`{id, revision, capture_digest}`; `context` is the observed
`{administration_revision, stage_revision, context_digest}` returned by the
precautionary context API. The server resolves the complete captured predecessor
and sources; clients do not supply actors, capture times or proof histories.

The closed action objects are:

- `{kind: "correct", values: {conditions, validity, supervision_text}}` corrects
  declared text while retaining the selected identity, measure class and actual
  judicial provenance.
- `{kind: "entered_in_error"}` marks the exact current record as entered in error.
- `{kind: "replace_entered_in_error", replacement_id, subject}` marks that record
  and creates its replacement under a new measure identity in one audited
  transaction. `subject` selects `{id, revision, values_digest}`. Both resulting
  records and their explicit replacement link belong to one original operation.

Submit accepts exactly `{command, expected_submission_digest,
expected_review_digest}`. Both confirmations are required. A successful submit
returns 201, including an exact retry that recovers an existing original capture;
the ports do not expose a separate newly-created flag. Recovery uses the original
operation UUID and preserves all original commitments. Reusing an operation UUID
for a different command is a conflict.

The server accepts JSON objects with a 128 KiB entity limit. Unknown or duplicate
fields, positional arrays, noncanonical UUIDs, nonpositive revisions and digests
other than 64 lowercase hexadecimal characters are rejected. Domain constructors
normalize accepted text; prepare returns that normalized command. The path case
and both confirmation digests are checked against the returned operation.

## Declared temporal values

Validity is `{start, statement, end}`. `end` must be present and may be null.
Each time is exactly one object selected by `precision`:

| Precision | Other fields |
| --- | --- |
| `unknown` | `reason` |
| `date` | `year`, `month`, `day`, `offset_seconds` |
| `minute` | Date fields, `hour`, `minute` |
| `second` | Minute fields, `second` |

Known times require an explicit nullable `offset_seconds`; unknown times require
an explanatory declaration. Missing precision and offsets remain unknown. The
API does not insert UTC, midnight or seconds, or infer expiry from the clock.

## Records, histories and pagination

Administrative responses contain `capture`, `origin` and `record_history`.
The capture preserves its review, actual one or two rows, replacement link,
recording time and digest. Its history contains earlier owners, excluding the
current operation. Origin preserves the original case, operation, submission,
review and capture commitments.

A measure response contains `case_id`, `reference`, `family`, `validity`,
`last_action`, `record_root`, `judicial_origin`, `last_judicial`, `record` and
`record_history`. Family `m1`, `m2` or `c1` identifies the actual original capture.
The history includes its complete owning group or operation and required sibling
records. An administrative replacement may establish a new administrative root
while preserving a separate judicial origin. Marked current records remain
visible; exact older revisions are not replaced by a later current head.

Both lists accept `limit` from 1 through 20, default 10. Administrative lists use
exclusive `after_operation_id`; measure lists use exclusive `after_id`. Items
are ordered by UUID. Responses include `case_id`, `items`, `has_more` and the
corresponding nullable `next_after_operation_id` or `next_after_id`. Unknown or
duplicate query keys are invalid. Other routes accept no query, except the
mandatory exact-read digest shown above.

The response boundary checks case, selectors, original commitments, owner and
member consistency before projection. Histories remain bounded to 256 owners
and 8192 rows, with bounded per-owner arrays; these structural checks do not
replace the application and PostgreSQL integrity checks.

Missing authorization yields 401 or 403; authorized absence yields 404;
confirmation, stale-head and known-dependant conflicts yield 409; incomplete
history yields 422. Invalid transport yields 400 and an oversized body 413.
Stored inconsistency yields a generic 500 without exposing database diagnostics.
See [the design decision](adr/0071-declared-precautionary-hearings-and-measures.md)
for the declared-record boundary, and [the verification report](verification-report.md)
for executed evidence and remaining acceptance work.
