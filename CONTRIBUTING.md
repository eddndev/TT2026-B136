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

The native document-format tests require Linux x86_64 and pinned qpdf 12.4.1.
Before running `cargo test --workspace`, prepare the library:

```bash
export TT_TEST_QPDF_LIBRARY="$(bash scripts/setup-document-formats.sh)"
```

`scripts/test-backends.sh` prepares it when unset; API and browser demos also
configure it for the server. Native parser tests must not silently skip when
it is absent. `serve` requires `DOCUMENT_QPDF_LIBRARY` or `--qpdf-library` and
checks the worker before accepting traffic. Follow
`docs/document-format-operations.md`; do not bypass admission on setup failure.

Continuous integration runs the same checks, plus a minimum-supported-Rust-
version check, coverage measurement, a dependency policy check (`cargo deny
check` against `deny.toml`), and a release binary size guard. See
`.github/workflows/ci.yml`.

## Documentation maintenance

Documentation is part of each functional delivery and belongs in the same
pull request as the behavior it describes. Update the affected documents:

- Keep `docs/http-api.md` aligned with routes, permissions, request and response
  formats, errors and examples. Update operational instructions in
  `docs/database-operations.md` and the relevant README when setup, migration,
  configuration, recovery or commands change.
- Record architectural rationale in `docs/adr/`. Preserve the context of older
  decisions and identify their replacements when behavior supersedes them.
- Record executed checks, environment, results and limitations in
  `docs/verification-report.md`. Distinguish fresh runs from historical evidence;
  do not refresh dates, counts or coverage without rerunning the corresponding
  checks. A skipped backend test is not evidence that the adapter was exercised.
- Keep project-state summaries and plans consistent with integrated behavior.
  Identify completed work and historical estimates before selecting subsequent
  work; do not treat a proposed schedule as proof of implementation.
- Keep the enforceable policies in `AGENTS.md` and `CONTRIBUTING.md` consistent.
  Preserve unrelated edits and deliverables when updating guidance.
- Keep local handoff records outside Git tracking. They may inform subsequent
  work, but versioned documentation, code and commits must not depend on them.

## Academic report maintenance

- Update affected implementation, testing and annex sections under `latex/`
  alongside functional changes. Describe implemented behavior, reproduced
  evidence and remaining limitations without claiming planned features are done.
- Preserve the approved abstract in `latex/frontmatter/resumen.tex`, objectives
  and state of the art unless their revision is explicitly requested.
- Integrate necessary technical corrections into the introduction and design
  as continuous academic prose. Do not append editorial notes about earlier
  versions, revision dates or a previous design. Keep diagrams, tables, captions
  and cross-references consistent with that prose. Dates identifying actual
  experiments and historical measurements remain part of the evidence.
- Keep `latex/chapters/06-conclusiones.tex` pending until the project is complete.
  Do not infer final conclusions from an intermediate backend delivery.
- Build changed report sources using `latex/README.md` and inspect the rendered
  PDF for legibility, pagination, unresolved references and missing glyphs.
  Record documentary checks in `docs/academic-report-verification.md` separately
  from software verification. Guidance-only edits do not require manuscript
  changes or a new report build when its sources are unchanged.
- Keep generated report PDFs untracked and publish them through the document
  workflow as artifacts and release assets. Preserve existing local deliverables
  before replacing them. The presentation is a separate deliverable and is
  updated only when included in the requested scope.

## Resuming work and comparing checkouts

- Recheck the active branch, working tree, and remote branch tips before using
  a historical status report. Code, migrations, and current test results take
  precedence over summaries and planned schedules.
- Before retiring a duplicate checkout, compare branch and tag tips, stashes,
  changed tracked files, untracked sources, and ignored runtime data. Equal
  HEAD commits alone do not prove that either directory can be discarded.
- Preserve existing document edits and generated deliverables when changing
  assistant guidance. Preserve the existing Qadra design system while implementing
  requested frontend changes; incomplete screens do not authorize an unrelated
  redesign.
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
  the workspace suite. Participant, administration and stage tests use isolated
  schemas in `CASE_TEST_DATABASE_URL`; document tests use
  `DOCUMENT_TEST_DATABASE_URL`.
  Missing variables cause the backend tests to return early.
- Report freshly executed checks separately from historical measurements in
  `docs/verification-report.md`. Run `scripts/demo.sh` for CLI changes and
  `scripts/api-demo.sh` for changes to the integrated HTTP workflow.

## Implemented project state

Reviewed on 2026-09-18. `docs/product-completion.md` is the detailed product map;
`docs/verification-report.md` distinguishes reproduced checks from pending work.
Inspect current code and contracts before treating this summary as complete.

- Case administration, documents and versions, manual/typed participants, stage
  transitions and hearing scheduling have authenticated, audited workflows.
- Declared hearing sessions/results include exact scheduling and continuation
  anchors, attendance, ordered agreements, provenance, correction, withdrawal and
  history in the backend, HTTP API and Qadra. They are verified locally; their
  declared content does not establish legal effects or activate deadlines. See
  `docs/hearing-results-api.md` and `docs/adr/0029-declared-hearing-sessions.md`.
- The global judicial calendar backend/API implements immutable revisions,
  declared scope and references, weekly rules, exceptions and exact civil-date
  classification. Owner manages; Owner, Litigator and Paralegal read; Client is
  denied. Audited persistence, restoration and Qadra are verified locally. See `docs/judicial-calendars-api.md` and
  `docs/adr/0030-versioned-jurisdictional-calendars.md`.

- Deadline profiles have immutable global or case-specific definitions, exact
  receipts, reproducible examples, audited PostgreSQL persistence and HTTP routes.
  Source revisions append durable change events in the same transaction. The
  pure evaluator checks applicability and verified inputs, preserving incomplete
  results without inventing a cutoff. These exact profiles feed persisted
  evaluations; source events do not yet have a processing worker. See
  `docs/deadline-profiles-api.md` and
  `docs/adr/0035-versioned-deadline-profiles-and-evaluations.md`.
- The `0017_` migrations persist case deadlines and immutable evaluation history.
  Register, correct, declare attention and retire commit state and audit together.
  Historical reads retain exact profile, source, calendar, responsible and result
  captures without reevaluation. The backend and HTTP API are integrated in
  `main`; their reproduced verification is in `docs/verification-report.md`.
  The current extension adds a case-authorized, audited selector of eligible
  active accounts and the Qadra workflow. Owner and Litigator manage; authorized
  Paralegal reads; Client is denied. Closure preserves reads and blocks writes.
  The selector does not grant membership or replace a general member directory.
  The full local Rust, HTTP/restore, browser and web verification campaigns
  have passed. Remote checks and integration have their own evidence. See `docs/deadlines-api.md`,
  `docs/adr/0036-persisted-deadline-evaluation-and-attention.md` and `web/README.md`.
  Automatic activation, reevaluation workers, combined hearing/deadline agenda,
  alerts and the qualified legal-profile acceptance corpus remain pending.

## Next work, in dependency order

Use `docs/product-completion.md` for acceptance scope and the corresponding
section in `AGENTS.md` for the dependency map. Current remaining work includes:

1. Complete integrated acceptance and publication of the Qadra deadline workflow.
   Qualify legal profiles with primary sources and acceptance cases, then implement
   durable activation, dependent reevaluation, combined agenda and notifications.
   Preserve exact inputs and immutable historical evaluations. Declared hearing
   text and civil classification do not establish legal effects.
2. Complete the remaining document, resource, identity, dashboard, report and
   audit-query use cases with explicit authorization and reproduced evidence.
3. Validate deployment limits, recovery and external dependencies; maintain the
   affected documentation and academic evidence under the rules above. Final
   conclusions remain pending until the complete project is finished.
