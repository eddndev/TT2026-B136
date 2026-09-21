# Reference password hashing cost

## Status

Implemented locally with seven focused tests and reference calibration passing.
Six CLI tests and the complete demonstration also passed. CI and the updated
PDF remain pending.
Executed results are recorded in `docs/verification-report.md`.

## Context

The target band is 500 through 1000 ms for one new credential hash. An isolated
five-run baseline with two passes averaged 400.3851816 ms, below that band, on
AMD Ryzen 7 7730U (16 logical CPUs), Linux 7.1.13 fc43. The development binary
optimizes the `argon2` and `blake2` packages at level 3. No other verification
runner was active. Historical averages of 529.4 ms and 409.6 ms retain their
original execution context; neither substitutes for this measurement.

Three passes averaged 659.7596048 ms across five runs with an explicit `inside`
verdict for 500--1000 ms. Build plus calibration took 79.514 s with 2553 source
files unchanged. The development binary SHA-256 was
`cf688da4940d70825b85c9d9d7ce26e3efc03888a0b206d7819e92268bdd5486`.
The 1/5/15-minute load averages were approximately 1.9004/2.6924/2.7114 before
and 2.043/2.626/2.641 after calibration, with 16 logical CPUs and one verification
runner. These values describe the execution context, not production load testing.

Six CLI tests passed in 20.916 s and `scripts/demo.sh` passed in 8.240 s, each
with 2553 source files unchanged. The demonstration separately measured 563.2 ms
across five hashes, also inside the band. This secondary reading does not
replace the primary 659.7596048 ms measurement or its binary identity.

## Decision

Set three passes for new Argon2id hashes. Keep memory at 262144 KiB (256 MiB),
one lane, a fresh 16-byte salt and a 32-byte output. CLI and server use the same
adapter; do not add an environment override or startup autocalibration.

Accept the reference calibration only from five measured runs with the inclusive
500--1000 ms band and an explicit `inside` verdict. A successful CLI exit alone
is insufficient. Record the binary profile, hardware and execution context;
repeat on a different deployment environment.

Verification continues to read each stored PHC's parameters. A historical
`t=2` value must accept the correct password and reject a mismatch with the
new constructor. Do not rewrite stored hashes or add a database migration or
implicit login rehash. Malformed PHCs remain errors, distinct from mismatches.

## Consequences

The focused tests require `m=262144,t=3,p=1` for new PHCs and preserve historical
verification. Seven tests passed in 21.602 s after the expected RED. The successful timing
measurement remains separate from these functional checks and applies only to
new hashes on this host and binary; historical PHCs retain their encoded cost.

Memory per hash remains 256 MiB, while another pass occupies the hashing worker
longer. Request concurrency limits remain unchanged. This band describes one
hash, not a whole login, enrollment or recovery attempt: the same adapter also
hashes recovery codes, and those workflows may perform several hashes.
Historical credentials keep their encoded cost until an authorized renewal
workflow replaces them. This change does not establish production load capacity,
recover accounts or complete certificate authentication.
