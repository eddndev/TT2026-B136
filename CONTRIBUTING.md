# Contributing

This document is the authoritative, versioned copy of the code standards for
this repository. Every rule below is enforceable in review: a change that
breaks one of them is not mergeable, independent of how useful it is.

## Character set

Source code, comments, identifiers, file names, configuration files, and
commit messages contain ASCII bytes only. No emoji, no Unicode punctuation
(no curly quotes, no en or em dashes, no non-breaking spaces), no accented
letters.

Markdown prose may use accented Spanish or English words, but the same bans
on emoji and fancy punctuation apply: straight quotes and plain hyphens only.

## File size

Every file stays under 400 lines. Split a module before it grows past that
limit; a file approaching it is a design smell, not a formatting problem.

## Test-driven development

Write the failing test first, then the code that makes it pass. Where
official test vectors exist (as they do for standard cryptographic
algorithms), those vectors are the first tests. Logic and its tests ship in
the same change.

## Self-contained comments and identifiers

Comments and identifiers must be understandable to a reader who has only
this repository: no chat logs, no planning documents, no ticket system.
Never reference plans, phases, milestones, iterations, tickets, external
section numbers, or conversations in code, comments, configuration, or
commit messages.

When code or configuration needs to cite a rationale, that rationale goes in
a versioned Architecture Decision Record under `docs/adr/` and is referenced
by its file path (for example `docs/adr/0001-msrv-and-dependency-pinning.md`).

## Commits and branches

- Commits follow Conventional Commits: `type(scope): summary`. The summary is
  in the imperative mood, lower case, ASCII, with no trailing period.
- One logical change per commit. Keep diffs small and reviewable; do not
  batch unrelated changes together.
- Stage explicit paths. Never use `git add -A` or `git add .`; the working
  tree may contain unrelated uncommitted work.
- Work on a branch, never directly on `main`. Branches merge into `main`
  with squash merge.
- Name new branches `feat/<description>` for implementation or
  `progress/<description>` for project documentation and coordination. Use
  descriptive names for the change and integrate through a pull request.

## Architecture: dependency direction

The workspace is a hexagonal architecture and the crate graph enforces the
dependency direction. Do not add a workspace dependency that violates it:

- `domain` has no dependency on any other workspace crate.
- `application` depends on `domain`.
- `infrastructure` depends on `domain` and `application`.
- `web` depends on `domain` and `application`.
- the binary crate (`despacho-cli`, in `crates/bin`) depends on all of them.

## Local verification

Before committing Rust changes, run:

```
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Continuous integration runs the same checks, plus a minimum-supported-Rust-
version check, coverage measurement, a dependency policy check (`cargo deny
check` against `deny.toml`), and a release binary size guard. See
`.github/workflows/ci.yml`.

## Resuming work and comparing checkouts

- Recheck the active branch, working tree, and remote branch tips before using
  a historical status report. Code, migrations, and current test results take
  precedence over summaries and planned schedules.
- Before retiring a duplicate checkout, compare branch and tag tips, stashes,
  changed tracked files, untracked sources, and ignored runtime data. Equal
  HEAD commits alone do not prove that either directory can be discarded.
- Preserve existing document edits and generated deliverables when changing
  assistant guidance. Treat frontend implementation as a separate requested
  task; the current placeholder does not imply permission to redesign it.
- For identity integration tests, set `IDENTITY_TEST_DATABASE_URL` and
  `IDENTITY_TEST_REDIS_URL` to isolated, disposable PostgreSQL and Redis
  instances. The tests in
  `crates/infrastructure/tests/identity_backends.rs` return early when these
  variables are absent, so an ordinary green test run does not prove those
  adapters were exercised. The PostgreSQL test expects an empty user table.
- For case integration tests, set `CASE_TEST_DATABASE_URL` to a separate,
  disposable PostgreSQL database. Do not reuse the identity test database:
  its bootstrap test requires an empty user table. `scripts/test-backends.sh`
  provisions isolated identity, case and document databases plus Redis and runs
  the workspace suite. Document tests use `DOCUMENT_TEST_DATABASE_URL`; missing
  variables cause the backend tests to return early.
- Report freshly executed checks separately from historical measurements in
  `docs/verification-report.md`. Run `scripts/demo.sh` for CLI changes and
  `scripts/api-demo.sh` for changes to the integrated HTTP workflow.
