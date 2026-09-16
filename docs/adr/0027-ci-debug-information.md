# ADR-0027: Bound debug information in CI artifacts

## Status

Accepted.

## Context

The workspace exercises many independent integration-test binaries. Full debug
information is duplicated across their links and increases both disk use and
cache size. The GitHub test and coverage runners exhausted available storage
while linking the expanded suite: their logs reported 0 MB and 83 MB free,
respectively, and linker processes terminated before test execution.

Discarding tests or coverage would remove required evidence. Deleting unrelated
preinstalled runner software would couple the workflow to an image layout that
the project does not control.

## Decision

The CI workflow sets `CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` to
`line-tables-only`. Compilation retains source locations for diagnostics while
omitting full variable and type debug information. The override applies only to
CI development and test artifacts; local defaults and the release profile stay
in their existing configuration.

All workspace tests, isolated backend services, native format checks and
instrumented coverage continue to run. The per-crate coverage thresholds and
dependency policy remain unchanged. Debug information is independent of the
coverage mapping emitted by Rust instrumentation.

## Consequences

Test artifacts consume less runner storage. CI stack traces retain file and
line locations, while interactive inspection of variables from those artifacts
requires rebuilding locally with full debug information. Cargo profile changes
invalidate the affected build cache. The workflow must still demonstrate that
the complete suite and coverage gates pass with the selected profile.
