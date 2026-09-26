# 0049: Parallel coverage on two repository runners

## Context

The workspace has many PostgreSQL integration tests. One instrumented Rust
campaign occupies the repository runner for hours, while formatting, linting,
the browser workflow, and the minimum Rust version check finish much earlier.
The execution measurements and failures are recorded in
`docs/verification-report.md`. Raising a job timeout cannot shorten the tests.
The repository also runs tests with disposable PostgreSQL and Redis services;
those services must remain isolated between concurrent jobs.

## Decision

Use two repository-specific Linux runners for the instrumented Rust campaign.
Each runner gets independent PostgreSQL and Redis services. The first runner
executes the non-infrastructure test executables and a smaller share of the
infrastructure executables; the second executes the rest. The assignment is
deterministic and disjoint. Recorded executable durations estimate the costly
groups, while unmeasured targets receive conservative default weights. Use a
2:3 relative capacity ratio when assigning those durations. This matches the
4:6 GiB memory limits and is more conservative than the 2:4 CPU quota ratio
because database waits do not scale directly with CPU. Keep
the ordinary Rust test harness within each executable, with at most two test
threads for the non-infrastructure crates. Infrastructure fixtures share
one PostgreSQL database even when their tables occupy distinct schemas.
Their audited writes and SQL guards acquire the same database-wide advisory
lock, and dispatch/worker adapters deliberately time out after one second
of lock contention. Run infrastructure tests with one harness thread so
unrelated fixtures cannot consume that timeout. Concurrency tests still
create their own threads and database connections; the two runners remain
parallel because their databases are independent. Keep the manual extended
dispatch measurement on the second runner as a separate job.

Each runner clears old raw profiles before its campaign and retains the new
profiles while running executables with `cargo llvm-cov --no-report`. This
option also retains compatible build artifacts. Generate LCOV only once at
the end, without a package filter, so the report contains the complete shard
even when its executables came from different crates. The gate
waits for both artifacts, unites covered source lines, and applies the existing
per-crate 90 percent threshold. It fails if either artifact is absent or
contains no workspace lines. The full suite remains mandatory; splitting it
must not reduce the counted coverage.

The repository is private before the second runner accepts work. Bound the
first runner and its rootless Docker daemon to two CPU cores and 4 GiB of
memory. Bound the second to four CPU cores and 6 GiB. Keep GitHub-hosted
checks on their existing parallel jobs; moving short checks to the new runner
would only queue them behind the long tests.

Both hosts must resolve Python 3.12 or newer even when `/bin/sh` starts with
an empty environment. Isolated format workers clear their environment, so a
runner-only PATH does not select their interpreter. Check this prerequisite
before compilation; provisioning instructions are in
`docs/ci-runner-operations.md`.

## Status

Accepted.

## Consequences

The two runners compile instrumented test binaries independently. The first
campaign on a cold runner can be slower, but later compatible campaigns reuse
their local build artifacts. Test completion is determined by the slower
shard. Historical timings will need adjustment if new slow executables make
the assignment uneven. A failed or missing shard prevents coverage approval.
The runtime limit and memory bounds protect neighboring services, but a
resource-heavy campaign can still increase their latency.
