# ADR-0068: Owner certificate proof followed by mandatory MFA

## Status

Accepted protocol direction. The structural statement is implemented and tested;
the typed internal cryptographic adapter is also implemented. The application,
live authority checks, storage adapters, session provenance and HTTP/UI
activation are not yet complete. No certificate login endpoint is enabled.

## Context

`docs/adr/0067-owner-certificate-bindings.md` records a public certificate binding
under an existing authenticated Owner. A historical registration receipt proves
neither current certificate validity nor possession of the private key now.
Reusing registration bytes for login would also allow a public historical proof
to substitute for a fresh challenge. Password authentication and its MFA workflow
must remain independently available.

## Decision

A certificate proof can be an alternative first factor only for its active Owner.
It must be followed by the existing mandatory MFA. The application will issue a
fresh, bounded, one-use challenge tied to the live binding, trust and account
revocation generation. Any change to the captured trust publication requires a
new proof. Individual document signing is a separate purpose.

The domain statement has exactly 182 bytes, with these half-open byte ranges.
Integers use big-endian encoding; UUIDs and fingerprints retain their raw bytes.

| Range | Value |
| --- | --- |
| 0..8 | ASCII `OWNAUTH1` |
| 8..9 | Version 1 |
| 9..10 | Purpose 1: first factor followed by MFA |
| 10..26 | Deployment UUID |
| 26..58 | Root SHA-256 fingerprint |
| 58..62 | Positive trust revision, u32 |
| 62..78 | Owner UUID |
| 78..86 | Authentication generation, u64 within the persisted i64 range |
| 86..102 | Binding UUID |
| 102..134 | Leaf SHA-256 fingerprint |
| 134..166 | Fresh 32-byte nonce |
| 166..174 | Inclusive issue instant, nonnegative i64 Unix seconds |
| 174..182 | Exclusive expiry instant, nonnegative i64 Unix seconds |

The interval lasts from one through 300 seconds, with checked subtraction.
Construction requires supplied active Owner facts and checked binding material;
it does not establish their current authority. The structural nonce constructor
copies exactly 32 bytes and makes no entropy claim. Clock, randomness, hashing,
private keys, signature verification and storage stay outside the domain type.

The account revision is intentionally absent: consuming a recovery MFA code
advances it. The application must still revalidate the full principal and its
unchanged revocation generation, binding and trust. A legitimate recovery update
must not be mistaken for a certificate revocation or excuse an unrelated change.

Before activation, the implementation must atomically consume a challenge before
RSA verification; recheck authority across expensive work and token creation;
bind subsequent MFA and sessions to certificate provenance and validity; cap
session deadlines; and invalidate challenges during restoration. Password-origin
sessions keep their existing semantics. An authority check admits an operation
at its checked boundary; this does not promise cancellation of work admitted
before a later withdrawal.

## Consequences

Registration and withdrawal statements cannot become login statements: their
purpose, byte layout and typed interfaces differ. Two independent complete
literal vectors and field/window boundary tests cover the structural format.
The adapter additionally verifies Partner proofs with the shared internal profile,
recomputes trust inspection, requires exact statement material and checks the
exclusive challenge window. It returns credential validity independently of the
nonce deadline. Its tests use an OpenSSL-generated proof, cross-purpose rejection
and a signed revocation. They do not establish freshness, one-use behavior,
current SQL authority, Redis expiration, MFA completion or session admission.
Those are required closing checks before exposing the alternative first factor.
