# evidence-verify bilateral contract fixtures

Public artifacts of one bilateral contract signature from a throwaway PKI,
used by the contract tests of `crates/evidence-verify`. They contain no
private keys: the keys exist only in a temporary directory while the
generator runs and are deleted when it exits. The fixtures of the server
seal and individual signing profiles live in `../fixtures` and are produced
by a different generator.

## Regenerating

From the repository root:

```bash
bash scripts/evidence-verify-contract-fixtures.sh
```

It needs bash, GNU `date`, OpenSSL 3.4 or later and `python3` (standard
library only; the helper is `scripts/evidence_verify_contract_fixtures.py`,
and the shell functions are shared with the other generator through
`scripts/evidence-verify-fixtures-lib.sh`). Before writing any statement the
helper rebuilds `crates/domain/tests/fixtures/contract_statement_vectors.json`
with its own encoder and stops if the bytes differ. The script deletes every
file in this directory except this README and writes a new set. Keys,
signatures and time-stamps are random or time dependent, so every run
produces different bytes: tests read identifiers, digests and instants from
the entries or from `fixtures.json`, never from hard-coded values.

The certificates are issued with `pki/openssl.cnf` (sections `v3_root_ca`,
`v3_internal_declaration`, `crl_ext`) and `pki/tsa.cnf` (`v3_tsa`,
`internal_tsa`), the configuration the deployment uses. The root issues the
party certificates, the time-stamping certificate and every CRL.

## Contents

Every file except `fixtures.json`, this README and
`parte-2-crl-cliente-revocado.pem` has the exact name of a package entry, so
the full package is the 26 entries plus `INSTRUCCIONES.md` and the partial
package is its 14 entries, as `docs/adr/0086-bilateral-contract-signing.md`
lists them.

| File | Meaning |
| --- | --- |
| `contrato.pdf` | PDF-shaped document of 70 KiB, over the 64 KiB limit of other entries |
| `contrato-solicitud.bin` | `CTRTREQ1` request naming both parties, their leaves and the validity |
| `ca.pem`, `tsa-chain.pem`, `parte-1-tsa-chain.pem` | The root, which is also the time-stamping root |
| `parte-N-certificado.pem` | Client (N = 1) and firm signer (N = 2), internal declaration shape |
| `parte-N-vinculo.bin`, `.sig` | `DOCBIND1` binding of each party, signed by that party |
| `parte-N-vinculo-raiz.der`, `parte-N-vinculo-crl.der` | Root and CRL at each binding registration |
| `parte-N-declaracion.bin`, `.sig` | `CTRTSIG1` statement of each party, signed by that party |
| `parte-1-declaracion.sig.tsr` | T1, over the SHA-256 of `parte-1-declaracion.sig` |
| `contrato.pdf.tsr` | Td, over the SHA-256 of `contrato.pdf` |
| `contrato-manifiesto.json`, `.tsr` | Contract manifest over the 23 covered entries, and Tf over it |
| `parte-1-crl.pem` | Root CRL whose window covers T1 |
| `parte-2-crl.pem` | Later root CRL whose `thisUpdate` is after the T1 interval and before Td |
| `parte-2-crl-cliente-revocado.pem` | The later CRL again, revoking the Client at its `thisUpdate`, before Tf |
| `contrato-recibo.json`, `contrato-acuse-parcial.json` | Application records of the full and partial packages |
| `fixtures.json` | SHA-256 and size of every other file, the request values and the instants |

The first CRL runs from the start of the generation hour minus one hour for
seven days, as `pki/gen-crl.sh` does. The later lists start two seconds after
T1 and also last seven days; the script waits until four seconds after T1
before requesting Td, so Td and Tf are covered by the later list and T1 is
not. Tokens use policy `1.3.6.1.4.1.13762.3`, accuracy one second, a nonce
and carry the signer certificate and the root.
