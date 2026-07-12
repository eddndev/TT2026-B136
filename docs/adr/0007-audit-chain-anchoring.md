# 0007. Audit chain anchoring

## Status

Accepted

## Context

The audit trail is a hash chain: entry `n` stores
`SHA-256(chain(n - 1) || canonical_bytes(entry n))`, with a fixed
all-zero genesis value for the first entry
(crates/domain/src/audit/chain.rs). Verification recomputes every link
from that genesis and compares it with the stored values.

This construction detects alteration of any entry, insertion of a
forged entry, and deletion or reordering anywhere before the last
entry: the first affected position no longer matches its
recomputation. It does not detect removal of entries from the tail.
Every prefix of a valid chain is itself a valid chain from the same
genesis, so an attacker who can rewrite the log file can drop the
newest entries and verification still reports the remaining prefix as
valid.

Detecting truncation requires comparing the stored head (the chain
value of the newest entry, together with its sequence number) against
a copy the attacker cannot rewrite: a value recorded outside the log,
for example kept by an operator, mirrored to independent storage, or
countersigned by a timestamping authority. The domain crate has no
storage or network capability by design - it defines ports and pure
logic only - so it has no place to keep such an external copy.

## Decision

The chain rule and `verify_chain` keep their current form, and their
documentation states the exact guarantee: tail truncation is outside
what the domain's chain verification can detect. Anchoring the chain
head outside the log is deliberately left to the persistence layer and
to operations: an adapter or deployment that needs truncation evidence
must record the newest chain value and sequence number in an
independent location and compare them before trusting a verification
result.

Two edge measures narrow the gap today without anchoring:

- `audit append` prints the assigned sequence number and chain value
  of every entry, so the operator's terminal or job log retains an
  informal record of the head over time.
- The CLI refuses to verify or show a log file that does not exist
  (crates/bin/src/audit_cmd.rs), so deleting the file outright - the
  crudest truncation - is reported as an error instead of as a valid
  empty chain.

## Consequences

- A "valid" verification result means the entries present form an
  unbroken chain from the genesis value. It does not mean the log is
  complete; the newest entries may have been removed.
- The threat model must treat an attacker with write access to the log
  file as able to truncate the tail undetected until the head is
  anchored externally.
- A future anchoring adapter (mirroring the head to independent
  storage, or obtaining a timestamped receipt for it) belongs in the
  infrastructure crate behind a domain port; the chain rule itself
  needs no change for that.
- Documentation and tests must not claim that deletion is detected in
  general. The honest claim is: alteration, insertion, and interior
  deletion or reordering are detected; tail truncation is not.
