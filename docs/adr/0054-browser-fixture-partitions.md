# 0054: Prepare browser fixtures for their owning partition

## Context

Playwright partitions tests after discovery, but each real-service job used
to prepare every fixture family first. Three jobs therefore enrolled the
same families of accounts three times in isolated databases. Real enrollment
hashes passwords and recovery codes using the unchanged Argon2id policy in
`docs/adr/0044-reference-password-hashing-cost.md`. Warm compilation removed
minutes of building without removing this duplicated setup.

## Decision

Use `scripts/web-live-plan.mjs` as the shared plan for discovery and fixture
preparation. `TT_WEB_LIVE_SHARD=1/3`, `2/3` or `3/3` assigns spec families to
three disjoint partitions. The live Playwright configuration selects those
files before importing their fixtures; do not additionally pass `--shard`,
which would partition the selected files again.

Keep related scenarios together: stage/adoption, hearing/alerts and
participant/typed-participant tests reuse their own prepared family. Keep
hearing setup before its alerts and Follow preparation before combined agenda.
Each job still has independent PostgreSQL, Redis, files, server and one
browser worker. All provisioning and authentication continue through the
real API, including unchanged password/recovery costs and permissions.

Assign administration and participant scenarios to the primary server's
third partition. The long administration workflow exceeded its unchanged
timeout on the shared support server, while the primary completed its
participant family alongside Rust. Move administration and its fixture
together. Keep deadline/agenda families on the first partition and
hearing-related families on the second. Validate the full union and concurrent runtime after moving
a family; a focused pass alone does not establish full-campaign stability.

Place document classification on that same third partition. Its complete
workflow performs three logins, concurrent metadata edits, two seals and a
final persisted-history reload. It exhausted the unchanged 60-second total
on the support server after earlier passing measurements near 50 seconds;
the final history request was still loading and its only recorded API error
was the expected metadata conflict. The scenario creates its own case and
uses reserved bootstrap recovery codes 2 through 4, so this move needs no
additional fixture family. Keep the ordinary document workflow on partition
one and retain every assertion and timeout. The placement addresses a
measured lack of timing margin; it does not establish a specific resource
bottleneck or improved runtime until the concurrent campaign is measured.

Stage transitions and adoption also belong to the third partition, together
with their shared case-stage fixtures. The complete transition scenario uses
concurrent authors, sealed historical evidence, closure and a final login. It
passed in 37.102 seconds on the support host and later exhausted its unchanged
60-second total during that final password request. Move the complete family
to the primary while retaining all three logins, every assertion and the
original timeout. This addresses insufficient timing margin on the support
host; concurrent acceptance must still establish stability and total duration.

When stage fixtures exist, their Owner uses reserved recovery code 7 to
provision optional families. Otherwise bootstrap code 6 remains available
because stage setup did not consume it. Bootstrap codes 0 through 4 remain
reserved for document scenarios, code 5 for administration and code 7 for
participant setup. All indices belong to the issued eight-code set; validate
the selected code before login. Provisioning does not create unrelated stage
accounts solely to obtain an Owner session. Extract hearing setup without
changing its operations.

Discover supported spec/test extensions recursively. A future unclassified
file gets one deterministic owner and conservative complete fixture setup;
it is never dropped for lacking a mapping. No partition setting retains the
complete setup for local unpartitioned runs. Invalid settings fail clearly.

Compare the union of actual Playwright listings against the previous complete
JUnit inventory. Verify full execution and gates remotely before declaring
the optimization successful. Log each fixture family's duration. Keep all
test assertions, timeouts, security settings and first-failure cancellation.

## Status

Accepted for measured rollout; refines the partitioning in
`docs/adr/0051-browser-ci-shards.md`.

## Consequences

Current fixture families are prepared once across the campaign, instead of
once per job. Tests remain isolated between jobs. Unequal family cost can
still leave unequal durations; use measured preparation and browser times to
adjust ownership. A successful inventory comparison proves selection, not
service correctness or runtime improvement. New scenario dependencies must
be added to the plan along with their tests.
