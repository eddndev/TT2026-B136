# 2. RSA signing crate and accepted advisory RUSTSEC-2023-0071

## Status

Accepted.

## Context

The prototype needs to produce RSA PKCS#1 v1.5 signatures over SHA-256
digests. The signing keys are issued by an internal certificate authority
operated with OpenSSL, so the signing code must interoperate with keys and
certificates produced outside the Rust toolchain.

The `rsa` crate from the RustCrypto project integrates directly with
`x509-cert`, `sha2`, and PEM handling, and verification works symmetrically
against a `SubjectPublicKeyInfo` extracted from a certificate. This keeps
signing and verification inside one coherent ecosystem of crates.

## Decision

Use the `rsa` crate for RSA signing and verification.

## Consequences

- Accepted risk: advisory RUSTSEC-2023-0071, a timing side channel in the
  private-key operations of the `rsa` crate known as "Marvin", has no
  patched release as of mid-2026. The risk is accepted under this threat
  model: signing runs locally through the command line, keys live on disk,
  and there is no network-observable timing oracle an attacker could
  measure.
- The advisory is registered as an ignore in the dependency-audit
  configuration (`deny.toml` and `audit.toml`), and those ignore entries
  point back to this record.
- Re-evaluate this decision when a constant-time release of the `rsa` crate
  ships. Note that such a release also raises the minimum supported Rust
  version, so it interacts with
  `docs/adr/0001-msrv-and-dependency-pinning.md`.
- Fallback: if the accepted risk becomes unacceptable before a fixed release
  exists, switch the signing implementation to `ring`.
