# 0052: CI and Web on repository-owned runners

## Context

GitHub-hosted final checks can be refused before execution by account billing
or spending limits, even when the preceding self-hosted tests passed. The
repository has three Linux servers with separately bounded runner accounts.
Browser partitioning and its required aggregate checks are described in
`docs/adr/0051-browser-ci-shards.md`.

## Decision

Run every job in the CI and Web workflows on repository-owned runners. Keep
GitHub Actions as the scheduler and artifact store; do not enable paid hosted
execution as a fallback. Other workflows are outside this decision.

- The dedicated server retains one `tt-ci-dedicated` runner for Rust tests
  and coverage generation, with sixteen isolated test slots.
- The first support server has one `tt-ci-vps1` runner for native format,
  Clippy, MSRV, dependency, binary-size and aggregate checks. Two additional
  `tt-ci-mock` runners each execute a simulated-browser shard in the official
  Playwright Ubuntu image. Its version must match the locked NPM dependency.
- The second support server has three `tt-ci-live` runners, one per real
  browser shard, with independent workspaces, backend services and ports.
  Each job uses one compiler and browser worker. System dependencies are
  prepared by the operator; jobs do not run privileged package installation.

All runner services and rootless containers on a server share its runner
user's cgroup budget. Budgets are host totals, not allowances per runner.
See `docs/ci-runner-operations.md` for labels, provisioning and limits.

Keep native compiler outputs outside the checkout using
`scripts/prepare-ci-workspace.sh`. Sequential support checks reuse one target;
concurrent browser runners each have their own target directory. Cargo still
validates and builds the current sources before execution. Preserve registry,
browser and compiler caches across jobs. Temporary backend data remains in
the current workspace's disk-backed `output/tmp` and is removed by the
existing service cleanup traps. The binary-size check resolves its executable
from the configured Cargo target directory.

Preserve every test, coverage threshold, dependency policy and aggregate
condition. Collect a full campaign after migration, including time spent
queued, cold compilation, execution and coverage. Compare warm campaigns
separately; ownership of a runner alone does not establish a speedup.

## Status

Accepted for measured rollout.

## Consequences

CI and Web no longer depend on hosted-machine spending approval. Maintaining
the operating systems, browser dependencies, runner software and caches is
now an operator responsibility. Support servers also host other services;
shared CPU or memory pressure can offset gains from persistent caches. Extra
runners increase parallel scheduling without increasing the host budget.
The first run on new compiler targets is cold and can exceed steady-state
time. Measure memory pressure and OOM events before raising concurrency.

References: [Playwright Docker images](https://playwright.dev/docs/docker),
[GitHub job containers](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/run-jobs-in-a-container).
