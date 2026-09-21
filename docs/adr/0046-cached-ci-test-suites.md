# 0046: Cached coverage builds and grouped integration suites

## Status

Accepted.

## Context

The workspace discovered 537 integration executables. Many compiled the same
fixture modules independently. On the persistent runner, cargo-llvm-cov also
cleaned workspace build artifacts before collecting coverage, so keeping the
Cargo target directory alone did not preserve instrumented workspace builds.
Opening a deadline repository validates the complete schema and inventory;
repeating that operation for each seed record measures startup repeatedly.

## Decision

Register related integration modules in bounded suites with shared fixtures.
Keep each source file below 400 lines. Preserve independently registered tests
whose helpers contain tests, require their parent's imports, invoke their own
process, or use a shared database. Keep the extended measurement target and
all browser tests unchanged. `scripts/check-test-layout.py` rejects missing or
duplicate source registrations. New integration sources must be registered in
the crate manifest or exactly one suite under `tests/suites/`.

Run coverage with `scripts/ci-coverage.py` and cargo-llvm-cov 0.9.1. Cache builds
by compiler/tool version, Cargo manifests and lockfile, Cargo configuration,
profile and instrumentation flags, and the wrapper script. This avoids reusing
obsolete executable variants when the target inventory or build settings
change. Before each run, delete raw profiles, merged profiles, profile input
lists and the output report. Use `--no-clean` to preserve compatible binaries.
Publish only after successful tests and fresh raw profiles. Keep the 90 percent
per-crate coverage gate unchanged.

Measure maximum child resident memory during the ordinary single-job workspace
build. `scripts/ci-resources.py` conservatively selects one or two subsequent
compile workers inside the existing 5 GiB cgroup limit: reserve 2 GiB for
services and the runner, then budget twice the measured compiler peak plus
256 MiB per worker. Retain the largest observed peak across cached builds;
a missing representative measurement selects one worker. Keep two Rust test
threads and one suite. Local verification still uses one build job and one
test thread. The cgroup remains the hard resource boundary; the estimate is
not a guarantee about the size of every future test harness.

Reuse one explicitly opened deadline seed repository per fixture/scenario in
dispatch families, candidates, worker replay and related setup helpers.
Preparation and commit still use the production adapters. Each test keeps its
own schema, role and fixture lifetime. Explicit reopen, corruption, migration,
transaction and concurrency assertions remain unchanged; production startup
validation is not bypassed or cached.

## Consequences

The grouped workspace has 203 integration executables. Fewer links and repeated
fixture compilations should reduce cold compilation; subsequent compatible
runs can reuse instrumented builds. The first run after this layout change is
cold. No wall-clock improvement is claimed until the complete CI finishes.
Names inside grouped executables gain a source-module prefix. Use the suite
name with `--test` and an optional module filter to run a focused scenario.
A full suite still creates isolated schemas and checks their inventories;
connection reuse only removes redundant startup work during seeding.
