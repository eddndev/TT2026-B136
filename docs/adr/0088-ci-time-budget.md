# ADR 0088: Ten minute budget for pull request campaigns

## Status

Accepted on 2026-11-08 at the author's request. Amends ADR 0082 and the
coverage cadence that ADR 0049 assumed.

## Context

On 2026-11-08 a pull request waited about 17 minutes for its slower campaign.
The Rust test job took 17.3 minutes on the dedicated runner, 16.1 of them
running 6,435 tests under `cargo llvm-cov`. The mocked browser suites took
14.1 minutes, serially, against `astro dev`. Every pull request paid the Rust
job, including those that changed only `web/`, and every main push repeated
the 15 minute browser campaign that the pull request had already passed on the
same tree.

## Decision

1. The CI workflow ignores pull requests whose changes are all under `web/`.
   No job in it reads that directory; the Web workflow still runs for them.
   Pushes to `main` are not filtered, so deploy gate reuse (ADR 0081) still
   finds a main run for every commit. Such a push has no pull request
   evidence, so its `reuse` job also accepts the successful main push run of
   the first parent when every path changed since that parent is under
   `web/`. A parent run that has not finished yet is not evidence.
2. The dedicated runner runs the Rust tests without instrumentation on pull
   requests, main pushes and calls from the deploy workflow
   (`scripts/ci_nextest.py --no-coverage`). The coverage gate runs on a nightly
   schedule against `main` and on a manual dispatch with `coverage: true`, which
   also measures a branch before merging. The shared-runner fallback of
   ADR 0049 keeps measuring coverage on every run.
3. The mocked browser suites serve the build that their job already produced
   (`astro preview`, selected with `TT_WEB_MOCK_SERVER=preview`) and run with
   two workers. Local runs keep the dev server. The module diagnostics spec
   names dev-server sources, so CI runs it against the dev server in a
   separate step.
4. A pull request Web run whose browser suites all passed publishes
   `tested-web-tree-<tree>`. A main push looks it up with
   `scripts/ci_tree_reuse.py --kind web` under the rules of ADR 0082 and, when
   found, skips both browser suites. The browser modules are still built on
   every main push, because deploy gate reuse requires them on the main run.

## Consequences

A pull request that changes only `web/` waits for the Web campaign alone. A
pull request that changes Rust no longer pays for instrumentation. Merging an
up-to-date pull request reruns no test suite, unless it changed only `web/`
and the main run of its parent was still in progress.

The coverage gate no longer blocks merges or deployments. A change that drops
coverage below the gate is reported by the next nightly run, at most one day
later, and is fixed on `main`. A pull request that removes tests should be
dispatched with `coverage: true` before it merges.

The preview server differs from the dev server in that it serves bundled and
minified assets. The application routes inside a single page, so the mocked
suites visit the same URLs. A defect that appears only in development builds
is no longer caught by CI.
