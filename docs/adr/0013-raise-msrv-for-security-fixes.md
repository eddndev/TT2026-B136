# 0013. Raise MSRV for dependency security fixes

## Status

Accepted.

## Context

The workspace previously supported Rust 1.78. That floor became incompatible
with security-fixed releases in two dependency families used by the running
application:

- `time` fixes RUSTSEC-2026-0009 in 0.3.47, which requires Rust 1.88.
- `tokio-postgres` fixes RUSTSEC-2026-0178 in 0.7.18, and
  `postgres-protocol` fixes RUSTSEC-2026-0179 and RUSTSEC-2026-0180 in 0.6.12.
  These releases require at least Rust 1.85.

Keeping Rust 1.78 would require either ignoring known advisories or carrying
local backports of upstream security patches. Both choices would weaken the
dependency policy and increase maintenance risk.

## Decision

Raise the workspace minimum supported Rust version to 1.88. Pin `postgres` to
0.19.14 and `tokio-postgres` to 0.7.18, and require `time` 0.3.47 or newer.
Resolve the lockfile with Cargo's incompatible-Rust-version fallback so every
transitive dependency is compatible with the declared floor.

The continuous-integration MSRV job uses Rust 1.88 and is blocking. The
dependency policy keeps the advisories enabled; no ignore entries are added
for these vulnerabilities.

## Consequences

- Contributors and build environments need Rust 1.88 or newer.
- The database and date-time dependency chains include the upstream security
  fixes and pass `cargo deny check` without advisory exceptions.
- A dependency update must preserve both the blocking MSRV check and the
  dependency policy check.
- The in-repository stored-entry ZIP writer remains in place. Raising the
  toolchain removes one historical constraint on external ZIP crates, but the
  reviewed writer still avoids an unnecessary dependency tree.
