# evidence-verify test fixtures

Public artifacts from a throwaway PKI, used by the tests of
`crates/evidence-verify`. They contain no private keys: the keys exist only
in a temporary directory while the generator runs and are deleted when it
exits.

## Regenerating

From the repository root:

```bash
bash scripts/evidence-verify-fixtures.sh
```

It needs bash, GNU `date`, OpenSSL 3.4 or later (for `req -not_before`) and
`python3` (standard library only; the helper is
`scripts/evidence_verify_fixtures.py`). The script deletes every file in this
directory except this README and writes a new set. Keys, signatures and
time-stamps are random or time dependent, so every run produces different
bytes: tests must take digests, serials and instants from `fixtures.json`, not
from hard-coded values.

The certificates are issued with `pki/openssl.cnf` (sections `v3_root_ca`,
`v3_end_entity`, `v3_internal_declaration`, `crl_ext`) and `pki/tsa.cnf`
(`v3_tsa`, `internal_tsa`), the same configuration the deployment scripts use,
so they have the exact shapes of the strict profiles in
`crates/infrastructure/src/certificates/internal_profile/`. Before writing
anything, the script checks with OpenSSL that every token verifies like the
server verifier (`openssl ts -verify -digest ... -CAfile`), that the foreign
and other-digest tokens fail against the wrong anchor or digest, that the
chains verify and that the CRL revokes only the revoked holder.

## Contents

All certificates are RSA-3072 with exponent 65537 (except
`signer-rsa2048.der`), signed with `sha256WithRSAEncryption`, and valid from
2026-01-01T00:00:00Z to 2036-01-01T00:00:00Z.

| File | Meaning |
| --- | --- |
| `root.der`, `root.pem` | Root with the deployment subject (`Root` shape) |
| `foreign-root.der`, `foreign-root.pem` | Unrelated root with another subject |
| `server-signer.der`, `.pem` | `Partner` shape, issued by the root |
| `holder.der`, `.pem` | `Declaration` shape, issued by the root |
| `revoked-holder.der`, `.pem` | `Declaration` shape, listed in `root.crl.der` |
| `tsa.der`, `.pem` | `TimeStamping` shape, issued by the root |
| `foreign-tsa.der`, `.pem` | `TimeStamping` shape, issued by the foreign root |
| `tsa-noncritical-eku.der` | Time-stamping certificate whose EKU is not critical |
| `signer-ca-true.der` | `Partner` shape except basic constraints CA true |
| `signer-rsa2048.der` | `Partner` shape except a 2048-bit key |
| `holder-with-eku.der` | `Declaration` shape plus an e-mail protection EKU |
| `root.crl.der`, `.pem` | Root CRL revoking `revoked-holder.der` |
| `root-empty.crl.der`, `.pem` | Earlier root CRL (lower CRL number), no entries |
| `foreign.crl.der`, `.pem` | Foreign root CRL, no entries |
| `document.bin` | Fixed document bytes |
| `server-signer.sig`, `holder.sig` | RSA PKCS#1 v1.5 SHA-256 signatures of `document.bin` |
| `certificate-binding.bin`, `.sig` | `DOCBIND1` statement for `holder.der` and `root.der`, signed by the holder |
| `document-consent.bin`, `.sig` | `DOCSIGN1` statement over `document.bin` and `holder.sig`, signed by the holder |
| `token-ess-v1.tsr` | Response over `document.bin`, ESS `SigningCertificate` (SHA-1) |
| `token-ess-v2.tsr` | Response over `document.bin`, ESS `SigningCertificateV2` (SHA-256) |
| `token-bare-v2.der` | Bare token (ContentInfo) over `document.bin`, ESS v2 |
| `token-other-policy.tsr` | Response with the non-admitted policy `1.3.6.1.4.1.13762.4` |
| `token-foreign.tsr` | Response signed by `foreign-tsa.der` |
| `token-other-digest.tsr` | Response over a digest that is not `document.bin`'s |
| `fixtures.json` | SHA-256 and size of every other file, and the facts below |

Every CRL has the same window: `thisUpdate` is the start of the generation
hour minus one hour and `nextUpdate` seven days later, as `pki/gen-crl.sh`
does; the revocation in `root.crl.der` is one day before `thisUpdate`. Tokens
use policy `1.3.6.1.4.1.13762.3` (except the other-policy one), accuracy one
second, no ordering, a nonce, and carry the signer certificate and its root.
Their `genTime` is the generation time, inside every CRL window.

`fixtures.json` records, per certificate, the serial (uppercase hexadecimal
without leading zeros), validity in Unix seconds, subject, issuer, key
identifiers, key size, intended profile and deliberate defect; per CRL, the
window, CRL number and revoked serials with their revocation instants; per
token, `genTime`, accuracy, policy, ESS kind, covered digest, signer and
anchor; and the decoded fields of both statements. Rationale for the checks
these fixtures exercise: `docs/adr/0085-in-browser-evidence-validation.md`.
