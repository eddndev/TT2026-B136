# 1. Minimum supported Rust version and dependency pinning

## Status

Accepted.

## Context

The project targets a minimum supported Rust version (MSRV) of 1.78,
declared with the `rust-version` field in `[workspace.package]` of the root
`Cargo.toml`. Developers build with newer toolchains day to day, so nothing
on a workstation naturally catches a dependency that quietly requires a
newer compiler.

Several of the cryptographic crates this project relies on have released new
major lines that raise the required toolchain above 1.78. Without explicit
version bounds, adding one of these crates at its latest version would make
the workspace unbuildable on the declared MSRV, and the failure would only
appear in an environment that actually uses the 1.78 toolchain.

## Decision

Pin the cryptographic crates to the release lines that build on Rust 1.78,
which is the family built around `der` 0.7:

- `x509-cert` 0.2.x
- `der` 0.7.x
- `cms` 0.2.x
- `x509-tsp` 0.1.x
- `rsa` 0.9.x

Pin `ring` to 0.17.12 or newer, because that release fixes the advisory
RUSTSEC-2025-0009.

These versions are fixed in `[workspace.dependencies]` of the root
`Cargo.toml`, so every member crate that opts in receives the same
compatible set.

## Consequences

- A naive `cargo add` from a member crate would pull `x509-cert` 0.3 and
  `rsa` 0.10, which require a newer toolchain and would break the
  minimum-version build. The workspace-level pins prevent that: member
  crates must consume these dependencies through
  `dep.workspace = true`.
- The workspace stays buildable on Rust 1.78, which continuous integration
  verifies with a dedicated check on that toolchain.
- The pinned lines will age. Revisit this decision if the minimum supported
  Rust version is later raised, at which point the newer major lines become
  available.
