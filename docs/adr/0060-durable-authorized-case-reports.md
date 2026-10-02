# 0060 - Durable authorized case reports

## Status

Accepted. Application, persistence, protection, renderer, HTTP, interface and
supervised composition have focused evidence. Integrated API restoration and
real-browser acceptance pass locally. Resource measurements distinguish a
representative PDF from a rejected maximum-size capture. Global closing checks
and deployment remain separate from these results.

## Context

The report workflow in `latex/chapters/03-analisis-diseno.tex` requires filters,
background generation and notification. The authorized dashboard provides live
aggregate indicators, not a durable export or a historical case census.
A long render must not hold the shared audit transaction. A worker restart or
membership change must not publish a partial or newly unauthorized artifact.

## Decision

Provide a case-state report with workload derived from the same captured cases.
Owner requests office scope; Litigator requests assigned-case scope. Other roles
are denied. Reports belong exclusively to their requester, including among Owners.
The original complete principal, account revision and authorization generation
remain attached to the request; changing and restoring a role cannot revive it.
The current full principal and authorization generation must match exactly. The
current account revision must be at least the captured revision: consuming a
recovery code advances that revision without changing the authorization generation.
Such credential use preserves report access across sessions; a later access change
and restoration still fail the generation check. The original captured revision
remains immutable evidence and is never refreshed during replay, rendering or reads.

Filters select original case creation in a UTC half-open interval of at most
366 days, current administrative status and optionally an assigned Litigator.
They do not represent activity during that interval or status as of its end.
Artifacts must label this distinction. No legal effectiveness score is inferred.

The report-specific Litigator picker uses the same current membership predicate
as the report filter. Owner sees all active Litigators, including those without
cases. Litigator sees active Litigators sharing any case, including closed cases.
Other roles are denied even for empty results. Each audited page reauthenticates
the complete principal, orders by user UUID and provides a bounded cursor with
no total count. The dashboard workload is not the report selector's authority.

An operation UUID plus a canonical digest makes exact replay return the same
report. Reusing that operation with changed filters fails. Request time is not
part of that digest. A captured snapshot binds report ID, requester stamp, scope,
filters, observation time, ordered cases and derived workload using explicit,
versioned canonical bytes. JSON is an interchange view, not the canonical format.

Capture at most 1000 cases, 10000 assignment pairs and 1000 workload users.
Limit canonical captures to 8 MiB and each rendered artifact to 16 MiB. Over-limit
results fail with an instruction to narrow filters; totals are never truncated.
PDF and CSV consume the identical immutable capture, including on render retry.

A durable queue uses an expiring lease identified by report, attempt, random
lease token and generation. Capture, renewal, completion and failure compare the
entire unexpired fence. Rendering occurs outside audit/database transactions.
Completion revalidates access to every captured case and atomically publishes
both artifacts, Ready state and a single report notice. A lost fence cannot fail
or overwrite a newer attempt. Revoked access denies the whole capture, including
on download: an export is never silently reduced to the remaining accessible
cases. One serial consumer belongs to the server supervisor and participates in
coordinated shutdown; the database owns recovery of interrupted attempts.

Snapshots and artifacts use the existing authenticated cipher and envelope key
ports. Their additional authenticated data has an explicit version and binds
report UUID, payload kind and plaintext digest. Key buffers are zeroized. The
protector verifies the plaintext digest and input limits before releasing bytes.
No document identity, signature or legal timestamp is fabricated for an export.

The renderer uses typed PDF primitives, bounded layout and bundled licensed fonts,
with no HTML, URL loading, host font lookup or arbitrary template language.
PDF must preserve supported Spanish/Latin text and reject missing glyphs rather
than substitute or omit characters. CSV uses uniform RFC 4180 records and prefixes
untrusted text with an apostrophe to prevent spreadsheet formula evaluation.
Explicit rows preserve capture identity and filters even when no case matches.
PDF layout also caps output at 512 pages and one million glyphs. These budgets
apply together; a capture at the row-count limit need not fit arbitrary text
within the page or byte limits.

The Linux composition invokes a trusted private entry point once per format.
It passes a sealed in-memory capture, clears the environment, closes inherited
service descriptors and bounds CPU to 15 seconds, virtual address space to
512 MiB, wall time to 20 seconds and output to the artifact limit plus its fixed
protocol header. Core dumps and regular file growth have zero limits. The parent
owns termination and reaping of the process group, including failure paths.
This resource boundary is not a filesystem, network or privilege sandbox; it
does not authorize untrusted executables or arbitrary rendering code.

Report notices are distinct from legal deadline/hearing alerts. Only an explicit
acknowledgment records the first read time; navigation and download do not mark
notices read. Ready and Failed notices persist across sessions. Email delivery
requires an independently configured provider and is not implied by an in-app
notice or by this decision.

## Consequences

Exports remain reproducible evidence of an authorized observation, not live data,
a historical census, an external attestation or a substitute for source records.
Durable captures and artifacts increase encrypted storage and backup obligations.
Retaining the deployment encryption key is necessary for restoration.

Acceptance must cover exact replay, isolation, authorization changes during work
and download, expired/stale leases, restart with the same capture, simultaneous
publication of both formats, notice acknowledgment, backup/restore, literal PDF
and CSV injection, text extraction, pagination, visual layout and resource bounds.
Local adapter tests alone do not close the end-to-end report workflow.
