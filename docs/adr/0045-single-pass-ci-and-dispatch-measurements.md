# 0045: Single-pass CI and explicit dispatch measurements

## Status

Accepted.

## Context

The CI workflow ran the complete backend suite once with `cargo test` and
again with coverage instrumentation. Most of its runtime is PostgreSQL fixture
setup and schema/inventory validation. The dispatch measurement seeded 240
records by opening and validating a repository for each record. Its pagination
assertions do not require repeated startup validation.

## Decision

Reuse one opened deadline repository while seeding each dispatch campaign.
Keep an ordinary 21-record correctness test with page limits 1, 20 and 100.
Keep the 240-record measurement as an explicitly ignored test, available through
`workflow_dispatch` with `dispatch_measurement` or the command in
`docs/deadline-dispatch.md`. Preserve exact identifiers, totals, page counts,
query plans and timing output. Reopening and tamper tests remain independent.

Run the workspace tests once under `cargo llvm-cov` in the `Test` job on the
repository-scoped runner labeled `tt-ci-vps2`. Also build the ordinary workspace
and run documentation tests, which stable coverage instrumentation does not
include. Publish the coverage report as a run-specific artifact. The `Coverage`
job depends on successful tests and applies the unchanged per-crate 90 percent
line gate to that artifact; it does not rerun tests.

The runner has one listener, a dedicated unprivileged account and rootless
Docker. Place the runner service and its user services in the same capped
slice: three CPU equivalents, 4500 MiB memory high watermark and 5 GiB maximum.
Use one measured ordinary build job and one or two coverage build jobs, as
specified in `docs/adr/0046-cached-ci-test-suites.md`, with three Rust test threads
within the single suite, as specified in `docs/adr/0047-disposable-postgres-ci.md`.
Serializing every test underused the capped runner and prolonged database
fixture setup; the shared resource limits still bound the test workers.
Service containers use
random host ports and disposable databases, with 768 MiB for PostgreSQL and
128 MiB for Redis. Use disk-backed private temporary files. Keep the repository
private and its workflow token read-only. Rust belongs to the runner account;
do not use wrappers delegating to other server accounts. Keep Cargo artifacts
under the runner account's `.cache/tt-ci/target`, outside checkout cleanup.
The persistent runner does not run `Swatinem/rust-cache`: its post-job binary
pruning can remove a freshly installed `rustup` and break the next job.

Cancel superseded pull-request CI and Web runs. Keep push and manual runs in
separate concurrency groups so a manual measurement cannot cancel a PR gate.

## Consequences

The ordinary CI still exercises PostgreSQL/Redis adapters and the cryptographic
coverage gate. Coverage cannot pass from a previous run's report, and a failed
Test prevents its dependent gate from passing. Instrumentation is now part of
the test environment; ordinary compilation and doc tests remain separate.

The manual campaign is not a per-PR performance threshold. Its artifact is
measurement evidence, not a production latency promise. Repeated schema
creation elsewhere in the integration suite remains a cost to measure and
optimize independently; this change does not share mutable database state
between tests or remove startup validation from production adapters.
