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
job, using cargo-nextest 0.9.146 to schedule individual tests with sixteen active
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
compiler footprint. Legacy defaults remain two workers and 6 GiB. The dedicated
PostgreSQL container has a 6 GiB limit within that shared budget: at 2 GiB, the eight-slot campaign repeatedly reached its child cgroup
limit while the parent retained memory headroom. Legacy containers remain at
768 MiB. Runtime slots and compile workers are different budgets: compilation finishes before
Nextest starts executing tests.

The dedicated database service is subsequently replaced by a bounded native
service in `docs/adr/0055-native-ci-postgres.md`; the six-GiB child limit and
all slot isolation rules remain in effect.

## Status

Accepted for staged deployment. Host provisioning and a complete dedicated
campaign are required before activation is considered verified.

## Consequences

Individual tests from a long executable can occupy different slots without
replicating its compilation or weakening its assertions. Nextest runs each test
in a separate process, so process-local fixture caches are rebuilt per test;
the first complete campaign must quantify that cost. Four slots were the initial
bound; six and eight slots passed the full regression. With eight slots and a
6 GiB PostgreSQL limit, the measured parent CPU average was 5.44 cores, peak
charged memory was 21.35 GiB and there were no OOM, swap or memory-limit events.
Twelve slots passed the complete regression in 12m14s with a 7-core quota.
Measured CPU averaged 6.19 cores, PostgreSQL connections peaked at 46, and
no OOM or swap occurred. Nine MemoryHigh events appeared; peak charged memory
was 23.93 GiB, primarily file cache and reclaimable kernel memory.

Sixteen slots use the dedicated host's full eight-core quota and the existing
26 GiB parent limit. The complete Rust campaign passed in 11m54s, with
3049 passing tests, no OOM or swap and a 21.05 GiB charged-memory peak.
The twenty-second improvement over twelve slots shows diminishing returns. A future service
deployment must review the shared CPU budget. The helper supports one through
sixteen slots; Redis indices 0 through 15 fit its default database capacity.
Database wait sampling and a complete regression must confirm that extra
concurrency improves time without exhausting connections or creating failures.
Local verification still uses one slot.

A 10-20 minute end-to-end CI duration is a performance objective, not a measured
result or a timeout imposed before measurement. Track cold compilation, warm
compilation, test execution and reporting separately, using individual timings
to identify repeated fixture work. Full regression, coverage and browser gates
remain required. The extended dispatch measurement remains manual.

References: [Nextest slot semantics](https://nexte.st/docs/configuration/env-vars/),
[target runners](https://nexte.st/docs/features/target-runners/) and
[coverage integration](https://github.com/taiki-e/cargo-llvm-cov).
