# 0047: Non-durable PostgreSQL for isolated CI tests

## Status

Accepted.

## Context

The repository runner starts a fresh PostgreSQL service for each CI job and
removes it after the job. Integration tests create many isolated schemas and
run the complete migration and startup inventory repeatedly. The full test
job was cancelled at its 90-minute limit before reaching the remaining
integration executables. Its first instrumented compilation took 13 minutes
47 seconds; the completed test executables then consumed over 75 minutes.
PostgreSQL wrote frequent checkpoints with thousands of files while tests
created schemas. No test requires recovery from a host or database crash.
Transaction commit, rollback, isolation, and logical backup tests still run.

## Decision

For the disposable CI PostgreSQL service, disable `fsync`, synchronous commit,
and full-page writes. Extend checkpoint timeout to 30 minutes and allow 2 GiB
of WAL before a size-driven checkpoint. Apply settings before creating test
databases and verify them in a new connection. Keep schema isolation, startup
validation, permissions, and failure tests. These settings do not apply to
production or developer databases.

Use three Rust test threads within the existing shared three-CPU and 5-GiB
runner limit. Keep a single test job and a single coverage collection. Allow
up to 180 minutes for the first complete campaign after this change so a
slow run can produce a result and coverage report; tighten that limit after
measuring the completed run. Do not count a cancelled or timed-out run as a
passed regression.

## Consequences

A host crash can lose or corrupt the disposable test database. Each CI job
creates it from scratch, so no committed data depends on its recovery.
PostgreSQL still provides normal transactional semantics during the job.
Extra test concurrency may increase memory use; the shared cgroup remains the
hard limit, and the completed run must be checked for memory events and time.
The change is a performance hypothesis until measured in a full CI run.
