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
  provisions both databases and Redis and runs the workspace suite; missing
  variables cause the backend tests to return early.
- Report freshly executed checks separately from historical measurements in
  `docs/verification-report.md`. Run `scripts/demo.sh` for CLI changes and
  `scripts/api-demo.sh` for changes to the integrated HTTP workflow.

## Implemented project state

Reviewed on 2026-09-11. This is a starting map, not a replacement for inspecting
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
  download, and audit verification. The contract is in `docs/http-api.md`.
- `migrations/0001_identity.sql` persists users in PostgreSQL. Redis stores
  revocable opaque sessions, challenges, login limits, and TOTP replay claims.
  These are not JWT sessions. See
  `docs/adr/0012-revocable-sessions-and-rbac.md`.
- `migrations/0002_cases.sql` persists case metadata and current assignments.
  `crates/application/src/cases/` authenticates every operation; the PostgreSQL
  adapter filters detail and paginated lists by membership. Owners see all
  cases and manage assignments; litigators may create cases and are assigned
  automatically. Other roles read assigned metadata only. Creation and the
  creator assignment share a transaction. See
  `docs/adr/0014-case-membership-and-isolation.md` and
  `crates/web/src/cases.rs`.
- Owner, Litigator, and Paralegal have global document permissions. Client
  document access remains denied even when assigned to a case: document/case
  associations and resource authorization are still pending. See
  `crates/domain/src/identity.rs`.
- Documents remain encrypted local JSON records; audit events remain a
  separate file. Their writes do not share a transaction. The document
  workflow creates version 1 and has no listing, search, or version-history
  API. See `crates/application/src/documents/port.rs`,
  `crates/infrastructure/src/documents.rs`, and
  `docs/adr/0011-local-document-workflow.md`.
- `crates/bin/src/serve_cmd.rs` explicitly selects the local OpenSSL TSA.
  The external provider adapter and local stub remain available, but a live
  provider campaign is outside the current delivery. The local TSA is
  technical demonstration evidence, not an authorized PSC's NOM-151
  attestation. See `docs/adr/0009-local-timestamp-authority.md`.
- `frontend/src/pages/index.astro` and
  `frontend/src/components/Hello.svelte` are placeholders, without a product
  interface or an API integration.
- `latex/main.tex` includes implementation, testing, conclusions, and annexes.
  Inspect `latex/chapters/06-conclusiones.tex` before describing the document
  as complete: the reviewed working copy contains pending-content markers.
  The versioned Beamer presentation starts at
  `presentacion/presentacion.tex`.

## Next work, in dependency order

1. Associate every document with a persisted case and enforce current
   membership in every document use case, including evidence export. Test
   cross-case denial and same-session revocation before enabling Client
   document access. Define migration of existing encrypted local records.
2. Move document persistence and audit writes behind an explicit transaction
   boundary, including durable case mutation history. Add failure and
   concurrency tests proving that rejected mutations do not leave document
   state and audit history inconsistent. Preserve captured signature and
   timestamp evidence and authenticated encryption context.
3. Add authorized document listing, detail, search, and version history.
   Define immutable historical evidence and bind each encrypted version to
   its document identity. Model procedural participants, hearings, and
   deadlines separately from the existing user access assignments.
4. In a requested frontend task, implement login/MFA, case navigation, upload,
   sealing, verification, and evidence download against `docs/http-api.md`.
   Keep business rules and cryptography behind the application ports.
5. Before public deployment, review TLS, database pooling and asynchronous
   clients, request concurrency limits, backup/restore, and the RSA threat
   model in `docs/adr/0002-rsa-signing-crate-and-advisory.md`: that record
   assumes CLI-only signing, while the current router also exposes sealing.
   External audit-head anchoring remains an open limitation documented in
   `docs/adr/0007-audit-chain-anchoring.md`.
6. Complete conclusions from reproduced results, reconcile design and
   presentation text with implemented behavior, and refresh the verification
   report after functional changes. Rebuild and visually inspect any changed
   document or presentation using its versioned README instructions.

For documentation maintenance, distinguish the stateless TOTP primitive in
`docs/adr/0008-totp-single-use-enforcement.md` from the Redis-backed replay
protection already implemented by the HTTP identity workflow. Likewise,
`X-Actor` in the historical document-workflow ADR was superseded by bearer
identity in `docs/adr/0012-revocable-sessions-and-rbac.md`; neither item is an
unimplemented HTTP authentication feature.
