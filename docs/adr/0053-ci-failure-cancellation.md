# 0053: Stop a CI campaign after its first failed check

## Context

A failed required check invalidates the revision. Continuing other suites
after that failure consumes shared runner resources and delays the corrected
revision. Matrix fail-fast alone does not stop unrelated jobs or the other
workflow in a CI/Web campaign.

## Decision

Enable fail-fast in the Rust and browser matrices. Nextest stops scheduling
tests after its first failure; Playwright uses one maximum failure per job.
Successful campaigns still execute every selected test and all coverage gates.

Each job has a failure-only final step using the local cancellation action.
After saving available diagnostics, it requests cancellation through the
Actions API for active CI/Web runs with the same head SHA, branch and event.
It cancels peer workflows before its own workflow. Completed runs, unrelated
workflows and other revisions are excluded. A manual measurement cancels only
itself. No periodic status polling or extra hosted runner is needed.

Grant `actions: write` for cancellation, retaining read-only repository
contents. The helper tolerates the API conflict response when a target has
already finished. It still attempts self-cancellation if peer lookup or a
peer cancellation fails, and reports other API errors. A job that cannot run
its final step because its runner is offline still requires operator cleanup.
Read-only fork tokens cannot perform cross-workflow cancellation.

Aggregate browser gates require every shard to succeed and are skipped when
the workflow has been cancelled. Cancellation never counts as successful
verification. Logs and any partial JUnit report explain the first failure;
they are not evidence that the complete test union passed. Full union and
coverage verification remain necessary on the corrected revision.

## Status

Accepted. Supersedes the collect-all-failures scheduling policy in
`docs/adr/0051-browser-ci-shards.md`.

## Consequences

Failures release capacity sooner, but later independent failures can remain
undiscovered until the next campaign. Cancellation is subject to API and
runner signal-delivery latency. The five helper tests check cancellation
scope, manual runs, completion races and API failures without cancelling
unrelated real jobs.

References: [GitHub matrix fail-fast and permissions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax),
[workflow cancellation API](https://docs.github.com/en/rest/actions/workflow-runs).
