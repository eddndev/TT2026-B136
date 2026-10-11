# 0076: Initial report estimates and durable ready email

## Status

Accepted for implementation. Public email activation requires explicit operator
configuration. Verification and integration are recorded separately in
[the verification report](../verification-report.md).

## Context

The existing state/workload and litigator-activity reports publish an immutable
PDF and CSV together. Their observed duration is known only after completion.
The interface also needs an initial generation estimate including queued work,
and an email notice when that initial estimate exceeds two seconds. Report
generation and email provider calls can both be interrupted or retried.

## Decision

Persist an optional initial total estimate and optional service estimate with
the original request. Repeating the operation returns those values unchanged.
Use the existing authorized case selection and metadata from the latest 64
reports completed in one attempt of the same type and authorization scope.
Exclude retried reports so their backoff is not counted as service cost. At least three
nonzero service samples within 25 percent of the requested case volume are
required. Scale each duration by `(requested cases + 1)/(sample cases + 1)` and
take the median. Do not extrapolate outside this neighborhood or fabricate a
value when history is insufficient.

Forecast the existing serial consumer's UUID order, active work, retry
eligibility and one-second poll intervals. More than 100 predecessors or an
unknown predecessor cost makes the total estimate unavailable. A live predecessor
lease on attempt two or later also makes the total unavailable because the
immutable capture does not identify that attempt's start; preserve any known
service estimate separately. Future arrivals,
unexpected failures and host load cannot be predicted. Capture the initial
prediction before processing; never replace it with observed completion time.
Validate independent requests with the fixed estimator: at least 90 percent
must differ from actual request-to-ready duration by at most the greater of
two seconds and 30 percent of actual duration. Report estimate coverage and
unpredicted requests as well as the accuracy denominator.

For an initial estimate strictly greater than 2000 milliseconds, insert one
durable email intent in the transaction publishing Ready, both artifacts, the
internal notice and audit event. Exactly 2000 milliseconds does not qualify.
Existing reports without estimates receive no retroactive mail intent. Mail
failure does not remove available artifacts or change their captured content.

Use a separate optional, supervised consumer. Its first claim persists the
complete generic envelope and a stable delivery UUID. Recheck the original
requester's identity, authentication generation and access to the complete
capture immediately before provider I/O. No report content, attachment,
credential or public download link appears in the message; its only link opens
the authenticated reports entry. Access revocation stops further attempts.

Lease each attempt for at most 30 seconds, with one provider call bounded to
10 seconds. Preserve message, recipient, delivery UUID and the original
24-hour provider-idempotency window across failures and restarts. Retry an
uncertain result after 60 seconds only within that window. After exhaustion,
retain a terminal uncertain outcome. Fence late results by delivery, lease,
generation and time. Provider acceptance is not proof of inbox delivery; no
exactly-once guarantee is claimed beyond provider idempotency.

## Consequences

Fresh installations honestly display an unavailable estimate until sufficient
comparable history exists. The estimate is a measured forecast, not a deadline
or a guarantee that every accepted capture fits rendering budgets. The existing
report limits, permissions, filters, counters and immutable downloads remain.

The database backup includes delivery state and persisted envelopes as well as
reports and audit history. Restoring a delivery does not reset its UUID or
provider window. Operational email is disabled by default and independent of
the captured-transport acceptance. Sender, public origin and credentials must
be configured before enabling it; enabling a channel can process eligible
pending intents that have not yet begun their provider window.
