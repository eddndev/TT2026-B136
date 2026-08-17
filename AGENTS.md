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
