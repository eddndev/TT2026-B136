# 0051: Browser CI shards with independent services

## Context

Browser checks are part of the required regression. Running all mocked API
tests on one worker takes twelve minutes. Real-service tests add fixture
preparation before their seven-minute browser campaign. The real-service
suite mutates shared accounts and cases, so increasing workers against the
same service instance would introduce interference.

## Decision

Split mocked browser tests into two Playwright shards and real-service tests
into three, on separate GitHub-hosted jobs. Preserve file-level scheduling and one worker per job. Each real-service
job provisions its own PostgreSQL, Redis, Rust server, identities and fixture
data through the existing disposable service script. No tests, assertions or
timeouts are removed, and no retries are introduced.

Keep the existing aggregate check names, `verify` and `Browser with real
services`. These checks run even when a dependency fails and require every
shard to succeed. Disable matrix fail-fast to collect the complete regression
result. Publish separate JUnit timing artifacts and failure diagnostics using
shard-specific artifact names.

The first validation enumerates the unsplit suite and every shard, verifying
that their sets are disjoint and their union is complete. Full remote runs
then validate both correctness and elapsed time. Local verification remains
one worker and one suite at a time.

On remote jobs, provision independent fixture families in batches of two.
Each family retains its sequential commands, accounts and case boundaries;
hearing-dependent alerts and the Follow/combined-agenda setup retain their
ordering. Wait for every task in a failed batch before logging out the shared
provisioner, and never start later batches after failure. The default remains
one provisioning worker for local execution. Emit per-family durations.
Browser builds use line-table debug information as described in
`docs/adr/0027-ci-debug-information.md`. Cache workspace crates as well as
dependencies with a separate browser workspace cache key. Cargo still checks
the selected revision and compiles before starting services; cached binaries
never substitute for a fresh test run.

## Status

Accepted for measured rollout.

## Consequences

The elapsed time can decrease while total runner minutes increase because
setup and fixtures are repeated. File-level partitioning preserves ordering
within each test file but may leave unequal durations; use JUnit timings to
evaluate balance. A successful partition listing alone proves neither browser
correctness nor an improvement in duration.

Reference: [Playwright sharding](https://playwright.dev/docs/test-sharding).
