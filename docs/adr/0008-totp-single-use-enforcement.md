# 0008. TOTP single-use enforcement deferred to the persistence layer

## Status

Accepted

## Context

RFC 6238 section 5.2 requires that a verifier must not accept the
second attempt of a one-time password after a successful first attempt
with it: an accepted code must be consumed.

The TOTP port (`TotpProvider` in crates/domain/src/crypto/totp.rs) and
its adapter (`TotpRsProvider` in crates/infrastructure/src/totp.rs)
check a submitted code against a given unix timestamp, accepting the
current thirty-second time step plus one step of clock skew on each
side. Verification is a pure computation over the secret, the code,
and the timestamp: no record is kept of the last accepted time step,
so a code that has been accepted once stays acceptable for the rest of
its window, about ninety seconds.

Consuming a code requires remembering, per secret, the highest time
step already accepted, and rejecting any code whose step does not
exceed it. That memory must survive process restarts and be shared by
every process that verifies codes, otherwise the check only pretends
to enforce single use. The prototype has no persistent store for
per-account authentication state yet.

## Decision

Keep `TotpProvider::verify` stateless and do not simulate a single-use
check with in-process state. An in-memory last-step map would be lost
on restart and not shared across processes, giving the appearance of
replay protection without its substance.

Single-use enforcement belongs to the caller of the port, once a
persistent store exists: record the time step of each accepted code
and reject any later submission whose step is not strictly greater
than the recorded one, as RFC 6238 section 5.2 prescribes.

## Consequences

- Residual risk: until the persistence layer lands, a code observed by
  an attacker can be replayed for the remainder of its acceptance
  window (the matching step plus one step of skew on each side, about
  ninety seconds with thirty-second steps).
- The port contract stays free of storage concerns and the adapter
  remains a pure computation, which keeps it trivially testable
  against the RFC 6238 and RFC 4226 vectors.
- When a per-account store exists, the enforcement point is the use
  case that calls `verify`: it must persist the last accepted time
  step atomically with the acceptance decision, so concurrent
  submissions of the same code cannot both pass.
