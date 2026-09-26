# 0050: Individually scheduled tests with isolated backend slots

## Context

Assigning whole test executables to runners leaves the slowest executable on
the critical path. Infrastructure fixtures use different schemas but acquire
the same database-wide audit lock. Running those fixtures concurrently against
one database caused unrelated tests to exhaust a one-second lock budget.
Serializing each executable prevents that interference but leaves available
CPU unused. Four independent coverage jobs on one host would also duplicate
compilation and build caches.

## Decision

Provide an opt-in dedicated Linux x86_64 runner labeled `tt-ci-dedicated`.
The repository variable `TT_CI_DEDICATED=true` selects one workspace coverage
job, using cargo-nextest 0.9.146 to schedule individual tests with four active
slots. The default remains the two-runner configuration from
`docs/adr/0049-parallel-coverage-runners.md` until the dedicated host is ready.

Create three disposable PostgreSQL databases per slot: identity, cases and
documents. Give each slot a separate Redis database index. Nextest guarantees
that its global slot number is unique among active tests. A target runner
selects the corresponding backend URLs before executing the test process.
Discovery passes through without altering the test listing; an actual test
without a valid slot or backend configuration fails instead of skipping its
adapters. Test subprocesses inherit the same URLs. Slot databases are reused
sequentially, as backend databases were in the existing serial campaign;
fixtures retain their schema, role and cleanup boundaries.

Production migrations, audit keys, startup integrity validation and transaction
timeouts remain unchanged. Explicit concurrency tests still create their own
connections and threads within their assigned database. The ordinary harness
remains supported for local and legacy CI execution.

The dedicated campaign compiles the instrumented workspace once and schedules
all ordinary tests without partition filters or retries. It clears previous
measurement files, retains compatible binaries, and produces one full LCOV
report. Documentation tests remain a separate step. The coverage merger
requires the exact report count selected by the workflow and retains the
existing per-crate 90 percent gate. Failed or missing campaigns cannot approve
coverage. JUnit records individual durations and failures; successful test
output is not stored or printed individually.

The compiler selector may opt into four compile workers and a 24 GiB budget on
the dedicated host, still respecting a smaller cgroup limit and the measured
compiler footprint. Legacy defaults remain two workers and 6 GiB. Runtime
slots and compile workers are different budgets: compilation finishes before
Nextest starts executing tests.

## Status

Accepted for staged deployment. Host provisioning and a complete dedicated
campaign are required before activation is considered verified.

## Consequences

Individual tests from a long executable can occupy different slots without
replicating its compilation or weakening its assertions. Nextest runs each test
in a separate process, so process-local fixture caches are rebuilt per test;
the first complete campaign must quantify that cost. Four slots are an initial
bound, not a demonstrated optimum. The helper supports one through eight slots
for subsequent measured tuning. Local verification still uses one slot.

A 10-20 minute end-to-end CI duration is a performance objective, not a measured
result or a timeout imposed before measurement. Track cold compilation, warm
compilation, test execution and reporting separately, using individual timings
to identify repeated fixture work. Full regression, coverage and browser gates
remain required. The extended dispatch measurement remains manual.

References: [Nextest slot semantics](https://nexte.st/docs/configuration/env-vars/),
[target runners](https://nexte.st/docs/features/target-runners/) and
[coverage integration](https://github.com/taiki-e/cargo-llvm-cov).
