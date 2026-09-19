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

Run only one local verification suite at a time. A single coordinator owns
Cargo, test runners and demo services; helpers must not start them. Set
`CARGO_BUILD_JOBS=1` and `RUST_TEST_THREADS=1` for local Cargo verification,
and use one worker for browser suites. Do not overlap builds, test suites,
coverage or document builds. After an interruption, inspect live processes
before starting another run; a lost terminal handle can leave child processes
running. Keep completed evidence separate from interrupted runs.

When `/tmp` uses tmpfs, configure `TMPDIR` on disk before starting a local
campaign. Use `output/tmp` in the active checkout, after
confirming that checkout is on a disk-backed filesystem. From the repository
root, create the private directory before exporting it:

```bash
mkdir -p output/tmp
chmod 700 output/tmp
export TMPDIR="$PWD/output/tmp"
export CARGO_BUILD_JOBS=1
export RUST_TEST_THREADS=1
```

Keep this environment for the coordinator and its child processes, including
browser runners, and use one browser worker. Do not delete unrelated files in
`/tmp`, the checkout or the temporary directory. Record the temporary filesystem
with verification evidence; a resource-loading error alone does not establish
its exact cause.

Implement and verify one observable workflow at a time. During development,
use the failing test and focused checks for the affected behavior. Commit and
publish coherent checkpoints with their executed evidence and remaining checks;
do not require another full regression for each checkpoint or documentation
adjustment. Repeat checks only when a change, failure or unresolved concern
justifies it. Keep incomplete deliveries in a draft pull request.

Run the full regression once when closing the functional delivery, before
merging. CI may supply this evidence for the published revision; do not repeat
the same broad campaign locally just to wait for it again remotely. Address
failures with focused checks, then rerun the affected closing checks. Preserve
TDD and all required merge gates.

Closing a Rust delivery requires these checks, locally or in CI for the
revision being integrated:

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
  evaluations. Source events feed the local dispatcher and worker ports; their
  local server composition has real API and browser verification. See
  `docs/deadline-profiles-api.md` and
  `docs/adr/0035-versioned-deadline-profiles-and-evaluations.md`.
- The `0017_` migrations persist case deadlines and immutable evaluation history.
  Register, correct, declare attention and retire commit state and audit together.
  Historical reads retain exact profile, source, calendar, responsible and result
  captures without reevaluation. The V1 backend and HTTP API are integrated in
  `main`; their reproduced verification is in `docs/verification-report.md`.
  The Qadra workflow and case-authorized, audited selector of eligible active
  accounts are also integrated in `main`. Owner and Litigator manage; authorized
  Paralegal reads; Client is denied. Closure preserves reads and blocks writes.
  The selector does not grant membership or replace a general member directory.
  The integrated V1 delivery passed its local Rust, HTTP/restore, browser and web
  verification campaigns. The later human/HTTP V2 changes have separate focal
  evidence and are not covered by those historical runs. Remote checks and
  integration have their own evidence. See `docs/deadlines-api.md`,
  `docs/adr/0036-persisted-deadline-evaluation-and-attention.md` and `web/README.md`.
  Automatic activation and the qualified legal-profile acceptance corpus remain
  pending. Reevaluation is integrated through PR 34; alerts are integrated
  through PR 36 with their own acceptance evidence below.

- The combined `/api/v1/agenda` query and Qadra day, week and month views are
  implemented. One authorized, audited transaction combines hearing heads with
  verified operational deadline dates, with bounded candidate scans and explicit
  continuation. Historical reads and pending, retired or changed deadlines provide
  no operational date. Opening preserves the exact revision. Backend/HTTP,
  desktop/mobile browser and restored-API acceptance passed for the published
  checkpoint. PR 35 integrated this delivery into `main` as `c16b820`. See
  `docs/agenda-api.md`, `docs/adr/0038-authorized-combined-agenda.md` and
  `docs/verification-report.md`.

- The `0018_` migrations extend deadline storage with human V2 tracking captures
  and observations while preserving V1 history. The single human service and
  HTTP workflow now prepare and confirm V2: register and correct require explicit
  policies; attention and retirement retain tracking and historical calculation.
  Authorship captures the authenticated user and email, and reauthentication
  compares the complete principal. Notification-parent heads are resolved
  separately from the parent's historical revision in the selected notification.
  Historical reads reconstruct exact evidence without recalculation. Current
  detail and list reads check verified dependency heads under audited access and
  keep calculation history separate from operational freshness and due time.
  The human port rejects technical authors. This extension has local focal
  verification. Qadra V2 passed 88 Node tests and 36 controlled-HTTP browser
  cases, including eight new desktop/mobile cases. A separate real-backend
  browser campaign passed 25 cases, including two Follow scenarios at 1440 and
  390 pixels; six screenshots were visually inspected. The composed runtime
  passed 31 unit and four CLI-help tests; its 26 loop/stop/supervision tests are
  included in the 31. A real API campaign passed reevaluation, TERM/INT shutdown,
  restart and restoration of R1-R5. The final API repeat and CLI demonstration
  also passed; detailed results are in `docs/verification-report.md`.
  Closing verification passed and PR 34 integrated this delivery into `main` as
  `e2e6758`. See `docs/deadline-tracking-api.md`,
  `docs/deadline-tracking-receipts.md` and `docs/verification-report.md`.
- The `0019_` migrations and `PostgresDeadlineDispatchStore` persist paginated
  event expansion and recurrent legacy reconciliation. Jobs, cursor advancement
  and audit commit together; jobs reserve their operation against human writes.
  Startup and reconnection verify schema, runtime grants and historical inventory.
  Reconnection restores statement and lock budgets; corruption prevents reuse until
  repair or restoration. Eleven focal reconnection, atomicity and timeout tests
  passed. Candidate bounds and missing-job filters apply before materialization.
  The local `serve` runtime serializes dispatch and worker consumption; real API
  and browser campaigns exercised this composition. See `docs/deadline-dispatch.md`.
- The `0020_` migrations and `PostgresDeadlineWorkerStore` implement local durable
  consumption, technical revisions, verified no-change results and failed attempts.
  Completion, revision and audit share a transaction; failures retain their own
  audited attempts and stable operation identities. History resolves exact sources
  without current-head substitution. Retry delay never bypasses startup inventory
  checks: persistent corruption prevents reopening until repair or restoration.
  Focal catalog, real restore/retry and failure-classification tests passed, as
  did the consumer delivery's local regression. Their scope is recorded in
  `docs/verification-report.md`. The worker is composed locally in `serve`.
  Runtime verification passed 31 unit and four CLI-help tests, including the
  26 loop, stop and supervision tests. Real API verification covered reevaluation,
  TERM/INT shutdown, restart and restoration of R1-R5, including the final API
  acceptance. PR 34 integrated the verified delivery into `main`; consult
  `docs/verification-report.md`. The human HTTP contract represents
  V1/V2 history and technical provenance without accepting worker commands. See
  `docs/deadline-worker.md` and `docs/deadline-tracking-api.md`.

- Durable activity alerts, account preferences, the personal Qadra inbox and
  the scheduler/email consumers composed in `serve` are integrated into `main`
  through PR 36 as `8261c51`. Internal alerts and optional generic email preserve
  delivery state, retries and duplicate control. API/restoration, real-service
  browser acceptance and CI passed. See `docs/alerts-api.md` and
  `docs/verification-report.md`.
- Declared procedural resources and acts preserve exact resolutions, supports
  and history. Their API/restoration and real-service browser acceptance passed;
  PR 37 is integrated into main as eeba869 with CI passed. The subsequent associations
  to existing hearings/deadlines have local implementation and API/restoration
  acceptance, with real-browser acceptance passed and CI closure still pending. Qualified legal
  profiles, durable activation and the complete resource workflow remain work
  under `docs/procedural-resources-scope.md`.

## Next work, in dependency order

Use `docs/product-completion.md` for acceptance scope and the corresponding
section in `AGENTS.md` for the dependency map. Current remaining work includes:

1. Qualify the remaining legal profiles with primary sources and acceptance cases;
   implement durable activation and complete the resource workflow beyond declared
   records and links to existing activities. Reevaluation, the combined agenda
   and alerts are already integrated. Close the separate resource delivery using
   its own acceptance and CI evidence.
   Preserve exact inputs and immutable historical evaluations. Declared hearing
   text and civil classification do not establish legal effects.
2. Complete the remaining document, resource, identity, dashboard, report and
   audit-query use cases with explicit authorization and reproduced evidence.
3. Validate deployment limits, recovery and external dependencies; maintain the
   affected documentation and academic evidence under the rules above. Final
   conclusions remain pending until the complete project is finished.
