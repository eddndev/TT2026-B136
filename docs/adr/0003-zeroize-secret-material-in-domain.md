# 3. Zeroize secret material carried by domain values

## Status

Accepted.

## Context

Secret material handled by this project (one-time-password secrets, recovery
codes, data-encryption keys, passwords in transit) must be wiped from memory
as soon as it is no longer needed. The `zeroize` crate provides the
`Zeroizing` wrapper, which overwrites a buffer with volatile writes when it
is dropped and is the established way to express that intent in Rust.

The domain crate defines the outbound ports of the hexagonal architecture,
and some of those ports return secrets: enrolling a second factor produces
the shared secret and its provisioning URI, and issuing recovery codes
produces the plain codes that are shown to the user exactly once. The types
that carry those values are declared in the domain crate, so the wiping
guarantee has to be expressible there. The domain otherwise keeps its
dependencies to a minimum (`serde`, `thiserror`, `uuid`) and depends on no
other workspace crate.

## Decision

Add `zeroize` to the domain crate's dependencies and declare every
secret-carrying field of domain values as a `Zeroizing` buffer.

`zeroize` is acceptable in the domain because it is a small utility crate
with no I/O, no framework surface, and no transitive pull on adapters; it
only encodes the invariant "this value is wiped on drop", which is a domain
security property rather than an infrastructure detail.

## Consequences

- Domain values that carry secrets (`Zeroizing<Vec<u8>>`,
  `Zeroizing<String>`) wipe themselves when dropped, in every layer that
  handles them, without each caller remembering to do it.
- The domain dependency list grows by one crate. Reviewers should continue
  to reject any domain dependency that brings I/O or framework code.
- Third-party crates that receive copies of a secret (for example a
  one-time-password library computing a code) may still hold transient
  unwiped copies internally; adapters must prefer library features that
  zeroize and keep such copies short-lived.
