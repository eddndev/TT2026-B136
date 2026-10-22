# ADR 0082: Reuse pull request tests on main pushes of the same tree

## Status

Accepted on 2026-10-21 at the author's request.

## Context

A squash merge of an up-to-date pull request produces a main commit whose tree
is identical to the merge ref that the pull request campaign already tested.
CI on the main push still repeated the whole Rust test job, about 18 minutes on
the dedicated runner, and the next pull request waited 19 to 22 minutes behind
it on the single dedicated runner.

## Decision

The test job of a pull request campaign, after all of its test steps pass,
publishes an artifact named `tested-tree-<tree>` with the Git tree it tested.
On a push to `main`, a `reuse` job looks up that artifact for the pushed tree
with `scripts/ci_tree_reuse.py`. It accepts it only from a completed, successful
`pull_request` run of `ci.yml` from this repository whose artifact has not
expired. When such a run exists, the test job and the coverage gate that
depends on it are skipped; the run is named in the job output.

Format, lint, MSRV, dependency policy, binary size and the deployment backup
check still run on every main push. Missing evidence, an API error or any other
event runs the tests as before. Calls from other workflows never reuse.

## Consequences

Merging an up-to-date pull request no longer occupies the dedicated runner, and
the main campaign keeps the advisory and policy gates current. A main commit
whose tree differs from every tested merge ref, for example after merging an
outdated branch, still runs the full suite. Flaky failures that did not appear
in the reused run are not retried. The evidence expires after seven days.
