# AGENTS.md

Guidance for AI assistants working in this repository. These rules are
mandatory. This file is the repository-local entry point for assistant
conventions. The same enforceable rules live in `CONTRIBUTING.md`; keep both
consistent.

## What this repository is

Trabajo Terminal TT2026-B136 (ESCOM/IPN). Two independent bodies of work share
one repository:

- `latex/` : the written document (LuaLaTeX). May be edited concurrently by a
  separate assistant. Never stage or commit files under `latex/` unless the
  change is yours.
- The Rust workspace + web frontend : the software prototype.

The Rust workspace is a Cargo workspace of five crates arranged as a hexagonal
architecture. The dependency direction is enforced by the crate graph:

    domain          <- no dependency on any other workspace crate
    application     -> domain
    infrastructure  -> domain, application
    web             -> domain, application
    bin             -> domain, application, infrastructure, web

`bin` is the composition root and the only crate allowed to use `anyhow`.

## Hard rules

1. ASCII only. Every byte of source code, comments, identifiers, file names,
   and commit messages must be ASCII. No emoji, no Unicode punctuation, no
   accented characters in code or commits. (Human-facing prose in Markdown and
   LaTeX is exempt, but code and commits are not.)

2. Files stay under 400 lines of code. Split a module before it grows past
   that. This applies to Rust, scripts, and config with logic.

3. Test-driven development. Write the failing test first, then the code that
   makes it pass. Cryptographic and domain logic must ship with tests in the
   same change. Prefer official test vectors as the first tests when they
   exist.

4. Comments and names must be self-contained. A reader who has ONLY this
   repository (no chat logs, no external planning document, no ticket system)
   must fully understand every comment and identifier. If a comment would stop
   making sense once the current work session is over, do not write it.

5. Never reference anything that is not a versioned file in this repository.
   Forbidden in code, comments, and commit messages: plan documents, phases,
   milestones, iterations, service codes, risk codes, requirement codes,
   section numbers, tickets, and past conversations. If you need to justify a
   decision, write a versioned Architecture Decision Record under `docs/adr/`
   and reference that file by its path. Unversioned planning and local
   assistant files can inform the work but must never be referenced.

## Commits and branches

- Conventional Commits. Format: `type(scope): summary`. Types: feat, fix, docs,
  style, refactor, perf, test, build, ci, chore, revert. Summary in the
  imperative mood, ASCII, lower case, no trailing period.
- One logical change per commit. Keep diffs small and reviewable; do not batch
  unrelated changes into a single large commit.
- Stage explicit paths. Never `git add -A` or `git add .`; another assistant
  may have unrelated uncommitted work in the same tree.
- Work on a branch, never directly on `main`. Branches merge into `main` with
  squash merge.
- Name new branches `feat/<description>` for implementation or
  `progress/<description>` for project documentation and coordination. Use
  descriptive names for the change and integrate through a pull request.

## Where rationale lives

Design rationale that code or configuration needs to point at goes in a
versioned ADR under `docs/adr/` (Nygard format: Context, Decision, Status,
Consequences), numbered `NNNN-short-title.md`. Configuration comments and code
comments reference the ADR file path, never an external document.

## Local verification

The installed toolchain is newer than the declared MSRV. MSRV is declared with
`rust-version` in the workspace manifest and enforced in CI, not by pinning a
toolchain locally. Before committing Rust changes, run:

    cargo fmt --all
    cargo build --workspace
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings

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
  the workspace suite. Participant, administration and stage tests use isolated
  schemas in `CASE_TEST_DATABASE_URL`; document tests use
  `DOCUMENT_TEST_DATABASE_URL`.
  Missing variables cause the backend tests to return early.
- Report freshly executed checks separately from historical measurements in
  `docs/verification-report.md`. Run `scripts/demo.sh` for CLI changes and
  `scripts/api-demo.sh` for changes to the integrated HTTP workflow.

## Implemented project state

Reviewed on 2026-09-15. This is a starting map, not a replacement for inspecting
the working tree. The prototype has a working cryptographic backend and an
authenticated local HTTP workflow; the complete case-management web product
is still unfinished.

- The CLI and application services implement SHA-256, AES-256-GCM envelope
  encryption and KEK rotation, internal PKI and CRLs, RSA-3072 signatures,
  RFC 3161 timestamps, integral verification, evidence ZIP export, Argon2id,
  TOTP, recovery codes, and a hash-chained audit log. Entry points are
  `crates/bin/src/cli.rs` and `crates/application/src/lib.rs`.
- `crates/web/src/routes.rs` exposes bootstrap, login, MFA, logout, current
  identity, user creation, document upload, sealing, verification, evidence
  download, and audit verification. Document routes require a case UUID; the
  retired global document routes return 404. The contract is in
  `docs/http-api.md`.
- `migrations/0001_identity.sql` persists users in PostgreSQL. Redis stores
  revocable opaque sessions, challenges, login limits, and TOTP replay claims.
  These are not JWT sessions. See
  `docs/adr/0012-revocable-sessions-and-rbac.md`.
- `migrations/0002_cases.sql` persists case metadata and current assignments.
  `crates/application/src/cases/` authenticates every operation; the PostgreSQL
  adapter filters detail and paginated lists by membership. Owners see all
  cases and manage assignments; litigators may create cases and are assigned
  automatically. Other roles read assigned metadata only. Creation and the
  creator assignment share a transaction with their audit event. Membership
  changes revalidate the actor and commit their audit in the same transaction. See
  `docs/adr/0014-case-membership-and-isolation.md` and
  `crates/web/src/cases.rs`.
- `migrations/0007_case_administration.sql` adds immutable administrative
  revisions and a separate initial stage registration. Complete penal creation
  requires NUC, judicial case number, their authorities and offenses; it commits
  active R1, creator membership, initial Investigation and audit together. Basic
  creation remains a pending profile with R1 and no stage. Earlier baselines
  project R0 without fabricated provenance; completing them does not add a stage.
  Staff queries and commands revalidate current authorization and commit audit.
  Owner manages all; assigned Litigator manages, assigned Paralegal reads and
  Client retains only the four-field basic projection. Expected revisions guard
  edits and status changes; current NUC and judicial case number are separately
  unique, including closed cases. Administrative closure blocks document,
  participant and stage mutations while preserving reads, evidence and Owner
  membership changes. Common audited transactions explicitly use READ COMMITTED. See
  `docs/adr/0022-audited-penal-case-administration.md` and
  `docs/case-administration-api.md`.
- The three `migrations/0008_case_stage*.sql` files add immutable stage revisions,
  canonical values and sequence guards while preserving initial registrations.
  Complete profiles without a stage support explicit adoption with reason and
  exact documentary support. Investigation -> Intermediate -> Trial uses two
  ordinary transitions, exact content versions, declared dates/offsets and an
  independent stage revision. Preparation verifies integrity and PDF/DOCX format;
  the audited commit revalidates active profile, role, membership, head and the
  complete prepared evidence. Closure preserves authorized reads and history;
  revocation prevents later access. Owner manages all, assigned Litigator manages,
  assigned Paralegal reads and Client is denied. See
  `docs/adr/0023-audited-case-stage-transitions.md` and `docs/case-stages-api.md`.
- Support admission runs in one bounded Linux worker using mandatory qpdf 12.4.1
  and the DOCX profile in `docs/adr/0024-isolated-document-format-admission.md`.
  It preserves original content, does not render it or certify legal authenticity,
  and does not retroactively validate every general document upload. Setup and
  limits are in `docs/document-format-operations.md`.
- Identity challenges are consumed atomically before MFA verification; user
  creation authenticates the bearer token inside the application use case.
  `crates/web/src/runtime.rs` shares request and blocking-operation limits
  across the full API, retaining worker permits after request cancellation.
  See `docs/adr/0015-backend-concurrency-and-invariants.md` and
  `docs/backend-review.md` for the reviewed behavior and deployment limits.
- `CaseDocumentService` authenticates before preparation and again before
  committing. PostgreSQL revalidates the active role, membership and exact
  document/case association inside the transaction. Owner can access every
  case; Litigator and Paralegal need current membership, and Paralegal cannot
  seal. Client document access remains denied even when assigned. See
  `crates/application/src/documents/case_service.rs` and
  `docs/adr/0016-case-document-transactions.md`.
- `migrations/0003_case_documents_audit.sql` persists encrypted documents and
  a shared audit chain for document, case and identity events. Document
  mutations and their audit commit together; verification and export confirm
  access and audit before returning results. Sealed evidence and the document's
  case association are immutable. Upload creates version 1. Authorized document
  listing, detail, literal name search and sealed-state filters are available;
  queries confirm audit before returning metadata. `0004_document_versions.sql`
  adds immutable document roots and snapshots keyed by UUID/version. Append
  checks the expected current version, history uses a descending cursor, and
  content actions select an exact version. `0005_document_metadata.sql` adds
  immutable classification revisions with captured authors and SHA-256 canonical
  values, separate from file evidence. Classified upload commits content, metadata
  and both events together; current filters choose the latest classification.
  Migration and startup require UTF-8 PostgreSQL. See
  `docs/adr/0018-authorized-document-queries.md`,
  `docs/adr/0019-immutable-document-versions.md` and
  `docs/adr/0020-audited-document-classification.md`.
- `migrations/0006_case_participants.sql` persists case-local directory roots
  and immutable revisions, separate from accounts and memberships. Owner manages
  all directories; assigned Litigator manages and assigned Paralegal reads;
  Client remains denied. Replacements require the current revision. Status-only
  archive/reactivation preserves current text inside the audited transaction.
  Captured author UUID/email and values remain historical. Civil identity and
  external judicial accreditation remain outside the internal demonstration.
  See `docs/adr/0021-audited-case-participants.md`.
- The `0010_` migrations add represented identities, eleven typed profiles,
  explicit candidate reviews and mixed manual/typed history. Each typed revision
  binds an exact subject revision and immutable document supports. Mutations
  recheck case status, membership, revisions, candidate review and captured
  documents inside the audited transaction. Declared identifiers and candidate
  signals do not establish civil identity. See
  `docs/adr/0025-case-subjects-and-typed-participants.md`.
- Defenders and control judges require an externally signed internal declaration;
  other natural-person profiles may supply one. The API verifies public evidence
  and never accepts private keys. `pki issue --purpose participant-declaration`
  issues the internal profile; `credential-trust publish` requires an administrative
  PostgreSQL connection and publishes monotonic CRL revisions. Runtime reads
  captured trust and preserves historical evidence after status changes. This
  does not implement certificate login or per-account document signing. See
  `docs/adr/0026-internal-participant-declarations.md` and
  `docs/typed-participants-api.md`.
- User creation and recovery-code consumption also commit their PostgreSQL
  audit atomically. Redis challenges and sessions do not participate in that
  transaction. Failed audit writes trigger best-effort removal of newly
  created credentials without returning their tokens; failed cleanup relies
  on Redis expiry. Logout never restores a revoked session after audit fails.
- `database migrate --runtime-role` applies schema and grants using an
  administrative connection. `serve` uses an existing restricted role without
  DDL. `database import` inspects legacy files by default; `--apply` requires
  a complete document/case map and preserves ciphertext, captured evidence and
  historical audit hashes. `serve --data-dir` checks the preserved legacy
  source for a completed cutover. Offline file adapters remain separate from
  HTTP persistence. Follow `docs/database-operations.md` for migration,
  reconciliation and restoration, and `scripts/test-backends.sh` for isolated
  identity, case and document database tests.
- `crates/bin/src/serve_cmd.rs` explicitly selects the local OpenSSL TSA.
  The external provider adapter and local stub remain available, but a live
  provider campaign is outside the current delivery. The local TSA is
  technical demonstration evidence, not an authorized PSC's NOM-151
  attestation. See `docs/adr/0009-local-timestamp-authority.md`.
- `web/` contains the Qadra design system and Astro/Svelte application.
  It integrates login/MFA, case selection and creation, persistent document
  queries, classified upload, independent metadata and version history, sealing,
  verification and evidence download. Classification conflicts preserve drafts
  and require explicit selection of the newer base revision. Actions use the
  selected content version; append conflicts preserve the chosen file.
  The staff case index offers current-profile filters and cursor pagination.
  Its summary distinguishes pending profiles, administrative status and initial
  stage; forms preserve drafts during revision conflicts and concurrent closure.
  Case navigation also exposes the participant directory, revision history and
  explicit conflict review for full edits and organizational status changes.
  Typed participant forms review candidate identities, select exact supports,
  export the binary declaration and receive public certificates and signatures.
  Subject edits preserve historical bindings; uncertain writes require explicit
  reconciliation before resubmission. Credential files reset when the selected
  identity, case or session changes.
  The Stages view separates current state, declared acts and initial history;
  it supports adoption, exact-version selection, transitions and reconciliation
  of conflicts or uncertain outcomes without automatic resubmission. Confirmed
  uploads survive a later stage rejection. Preserve its
  design tokens, components and original brand assets. `frontend/` retains the
  older placeholder; new product work belongs in `web/`. Browser mock tests and
  `scripts/web-demo.sh` against isolated real services provide separate evidence.
- `latex/main.tex` includes implementation, testing, conclusions, and annexes.
  Its implementation and testing distinguish the reproduced backend delivery
  from pending product and usability work. The approved abstract is preserved;
  conclusions remain pending until the project is complete. Update affected
  implementation, testing and annex sections as evidence changes.
  The versioned Beamer presentation starts at
  `presentacion/presentacion.tex`.

## Next work, in dependency order

`docs/next-goal.md` records the completed case-document delivery and retains
its original proposed schedule. Its reproduced evidence is in
`docs/verification-report.md`; the academic update is documented in
`docs/academic-report-verification.md`. Reassess historical estimates against
current code before planning subsequent work in this dependency order.
`docs/product-completion.md` tracks the broader product acceptance scope:

1. Reconcile permitted general-upload formats and content delivery before declaring
   the complete document use cases fulfilled. Current classification, queries
   and immutable versions preserve case isolation and Client denial. Delivery
   without a seal and security alerts to the Owner remain separate work; widening
   Client access requires an explicit tested resource policy.
2. Complete hearings, deadlines and resources linked to resolutions. Penal
   administration, stage adoption, two ordinary transitions and the mixed
   participant directory are implemented. Typed identities and internal
   declarations preserve reviewed provenance; they do not establish civil
   identity, external judicial authority or automatic deadline computation.
   `docs/procedural-resources-scope.md` preserves the approved objective and
   proposes a separate resource workflow, not a fourth linear stage. Keep these
   records separate from account assignments and organizational archiving.
3. Extend the Qadra interface with procedural workflows, user administration,
   dashboard aggregates, reports and audit queries against `docs/http-api.md`.
   Use a user directory for assignment selection instead of requiring raw UUIDs.
   Resolve certificate login and per-user signing identity before declaring
   those objectives complete. Keep business rules and cryptography behind ports.
4. Before public deployment, measure database pooling and asynchronous clients,
   request budgets, transport limits, TLS and graceful shutdown. Exercise
   backup/restore and Redis outages; design recoverable enrollment delivery
   and the remaining identity lifecycle across PostgreSQL and Redis. Review
   the RSA threat model in `docs/adr/0002-rsa-signing-crate-and-advisory.md`,
   whose CLI-only assumption predates HTTP sealing. External audit-head
   anchoring remains open in `docs/adr/0007-audit-chain-anchoring.md`.
5. Keep implementation, testing and annexes aligned with reproduced results,
   integrate design corrections into academic prose, and refresh the
   verification report after functional changes. Complete conclusions when
   the project is finished. Rebuild and visually inspect any changed document
   or presentation using its versioned README instructions.

For documentation maintenance, distinguish the stateless TOTP primitive in
`docs/adr/0008-totp-single-use-enforcement.md` from the Redis-backed replay
protection already implemented by the HTTP identity workflow. Likewise,
`X-Actor` in the historical document-workflow ADR was superseded by bearer
identity in `docs/adr/0012-revocable-sessions-and-rbac.md`; neither item is an
unimplemented HTTP authentication feature. The global document authorization
and separate online file writes described by the older ADRs are superseded by
`docs/adr/0016-case-document-transactions.md`; their offline behavior is not the
current HTTP persistence model.
