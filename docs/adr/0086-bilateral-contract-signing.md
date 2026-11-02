# 0086 - Bilateral contract signing

## Status

Accepted. This record fixes the contract entity, the signature request, the
two party statements, their time-stamps, both evidence package profiles and
the Client access rule before their implementation. It amends the
single-signer limitation of
[ADR 0085](0085-in-browser-evidence-validation.md), which stated that a
manifest with more than one signer is not supported until a later profile
defines multi-party completion and consent. This record is that later profile.
ADR 0085 is not edited: its individual signing profile, its manifest version 1
and the verdicts of every package it admits stay byte for byte as they are.
[ADR 0074](0074-individual-document-signatures.md) (individual signing) and
[ADR 0079](0079-browser-local-cryptography.md) (local keys) remain in force.

On 2026-11-02 the project authors authorized Client accounts to access their
own contract signature requests, exactly as the Access section below
describes. This is the explicit human decision that `AGENTS.md` requires
before Client access rights change; no other Client access is authorized by
it.

## Context

Individual signing (ADR 0074) lets one bound Litigator sign one exact PDF
version with a key held outside the server. The in-browser validator
(ADR 0085) admits that package and the server seal package, and rejects a
manifest with more than one signer. Nothing in the system lets two parties
sign the same document version in a fixed order with evidence that each party
can keep and check without Qadra.

A "contract" is today only a free-text document type in the classification
metadata of a document series (`crates/domain/src/document_metadata.rs`). It
can be reclassified, has no state, no history and no permissions of its own.

The Client role holds zero `Permission` (`crates/domain/src/identity.rs`).
Client document access is denied even when the Client is assigned to a case,
and `AGENTS.md` forbids inferring new access rights from internal roles.
Case membership currently accepts any active account, so an assigned Client
receives the basic case projection.

An earlier design of the contract workflow had the system generate the
Client's key and sign with the key of each party. Whoever generates or uses a
key can sign with it, so that design would let the firm sign for its own
counterparty, and the Client could rightly deny any signature. Non-repudiation
of origin assumes that only the holder of the private key could have produced
the signature.

The internal CA, the time-stamping authority and the server all belong to the
firm (`web/apps/qadra/src/lib/evidence/pinned-roots.json`). Evidence produced
under them is a technical statement of the firm, not a qualified third-party
attestation. The signature is a simple electronic signature with a local
RFC 3161 time-stamping authority; NOM-151, an accredited provider, external
anchoring and PAdES remain out of scope.

## Decision

### Contract entity and derived state

A contract is an immutable root bound to exactly one document series of one
case (one contract per series), with a title of 1 to 160 characters and its
captured author. It has no mutable state column. Its state is derived from the
last of its append-only events and from the `expires_at` of its last request:

| State | Condition |
|---|---|
| `borrador` | Last event `created`, or the last request closed without completion (`rejected`, `declined`, `cancelled`, `expired`), or the last request is still nominally open but its validity is exhausted. Admits a new request |
| `en_firma` | There is an open request |
| `firmado` | Last event `completed`. Terminal; an addendum is another contract |
| `abandonado` | Last event `abandoned`, reachable only from `borrador`. Terminal |

Every contract event carries the next revision of its contract (previous
revision plus one, starting at 1 for `created`). The primary key
`(contract_id, revision)` is the expected-revision lock; there is no mutable
row. Every event commits with its audit event in the same transaction.

**Open request.** This is the single definition used by the derived state,
by request registration and by the version guard: a request is open when the
last event of that request is `requested` or `client_signed` and
`now < expires_at`. A request whose validity is exhausted is not open, even
before its `expired` event is written, so a contract is never frozen by a
missing expiry consumer.

**Expired-closing rule.** When a new request is registered on a contract
whose previous request is exhausted but not yet closed, the same transaction
first writes the `expired` event of the previous request and only then the
`requested` event of the new one. The revision chain of a contract therefore
never holds an `expired` of request A after a `requested` of request B, and
the periodic expiry consumer only finds solitary exhausted requests.

A new version of the contracted document is not blocked, but signing a request
whose version is no longer the latest answers `contract_version_superseded`;
the request must be cancelled and a new one created. A sealed or
individually signed version cannot be contracted, and while a request is open
or completed no seal or individual signature may be written for its version
(`contract_version_frozen`). Bilateral evidence lives only in contract tables:
`documents.evidence` stays NULL for a contracted version, so a two-party
contract is never shown as a server seal.

### Signature request

A request names one exact PDF version, one Client account, one firm signer
account (an Owner, or a Litigator who is a member of the case), the
requesting account, the certificate leaf of each party fixed from their live
bindings, a validity of 1 to 30 days and the reference of the Client's
fingerprint sheet (`ceremony_reference`, 1 to 80 characters of declared text,
outside every signed byte). The Client signs first (position 1) and the firm
signer second (position 2). The order is fixed.

### `CTRTREQ1`: the request, 240 bytes

The server builds these bytes when it creates the request. Nobody signs them
directly; each party statement carries their SHA-256. Integers are unsigned
big-endian unless stated; UUIDs are their 16 raw bytes; digests are 32 raw
bytes.

| Offset | Length | Field |
|---|---|---|
| 0 | 8 | ASCII `CTRTREQ1` |
| 8 | 2 | `[1, 1]`: purpose 1 and format version 1 |
| 10 | 16 | Deployment |
| 26 | 16 | Request |
| 42 | 16 | Case |
| 58 | 16 | Document series |
| 74 | 4 | Document version, u32 |
| 78 | 32 | SHA-256 of the PDF content |
| 110 | 16 | Client account |
| 126 | 16 | Firm signer account |
| 142 | 16 | Requesting account |
| 158 | 32 | SHA-256 of the Client certificate (DER) |
| 190 | 32 | SHA-256 of the firm signer certificate (DER) |
| 222 | 8 | `created_at`: Unix seconds, signed i64 |
| 230 | 8 | `expires_at`: Unix seconds, signed i64 |
| 238 | 1 | Number of parties: 2 |
| 239 | 1 | Completion rule: 1, every party before `expires_at` according to its time-stamp |

`created_at` and `expires_at` equal the stored `created_at_seconds` and
`expires_at_seconds`; nanoseconds never enter the bytes. The layout is fixed
binary so that SQL can rebuild it and there is no canonicalization surface. It
holds no free text and no names: a counterparty name is shown from its
certificate and labelled as declared. The document series identifies the
contract, so the contract identifier is not part of the bytes.

### `CTRTSIG1`: the party statement, 303 bytes

| Offset | Length | Field |
|---|---|---|
| 0 | 142 | The shared prefix of `crates/domain/src/document_signing.rs` (`canonical_prefix`) with magic `CTRTSIG1`: `[1, 1]`, deployment, credential root SHA-256, trust revision (u32), account, account revision (u64), authentication generation (u64), binding, certificate leaf SHA-256 |
| 142 | 16 | Operation |
| 158 | 16 | Request |
| 174 | 32 | SHA-256 of the 240 `CTRTREQ1` bytes |
| 206 | 32 | SHA-256 of the PDF content |
| 238 | 1 | Position: 1 Client, 2 firm signer |
| 239 | 32 | SHA-256 of the position 1 signature (`parte-1-declaracion.sig`); zeros in position 1 |
| 271 | 32 | SHA-256 of the position 1 time-stamp response T1 (`parte-1-declaracion.sig.tsr`); zeros in position 1 |

Each party signs its 303 bytes with RSA-3072 PKCS#1 v1.5 and SHA-256, giving
a 384-byte signature. Magic and length keep both formats apart from every
other signed statement: `DOCBIND1` (142), `OWNCERT1` (150), `OWNAUTH1` (182)
and `DOCSIGN1` (258).

The codecs reject, in every copy:

- a length other than 240 or 303, another magic, or format bytes other than
  `[1, 1]`;
- a nil deployment, request, case, document, account, binding or operation;
  a document version of 0;
- in `CTRTREQ1`: a party count other than 2, a completion rule other than 1,
  a Client account equal to the firm signer or to the requesting account,
  equal certificate leaves, or `created_at >= expires_at`;
- in `CTRTSIG1`: the account counter ranges and the zero trust revision that
  the shared prefix already rejects, a position other than 1 or 2, a position 1
  statement whose chain digests are not all zeros, and a position 2 statement
  with an all-zero chain digest.

The 1 to 30 day validity window is a storage constraint, not a codec rule.
The copies that must agree, all tested against the single vector
`crates/domain/tests/fixtures/contract_statement_vectors.json`, are the domain
codec in `crates/domain/src/document_signing/contract.rs`, the independent
validator decoder in `crates/evidence-verify/src/contract_statements.rs`, the
SQL reconstruction and the browser values module in
`web/apps/qadra/src/lib/contract-signing-values.mjs`. The browser module
returns `created_at` and `expires_at` as decimal strings, like the 64-bit
account counters, so it accepts every signed 64-bit instant the Rust decoders
accept rather than only the JavaScript safe-integer range.

### Why no party signs the raw PDF bytes

Each party signs exactly one binary statement that contains the SHA-256 of the
PDF. A detached signature of the Client over the raw PDF would carry no
request, no counterparty and no validity. It would survive rejection,
cancellation and expiry, and `openssl dgst` would read it as "the Client signed
this PDF", defeating the signed `expires_at`. This is the structure of CMS
signed attributes: the signature covers a structure that contains the content
digest. The cost is that OpenSSL verification takes two steps: verify the
statement signature, then compare the digest at offset 206 with the PDF.
Direct cryptographic coverage of the PDF remains through the RFC 3161 token Td
over its SHA-256. Individual signing keeps its two signatures unchanged.

### Time-stamps

| Token | Imprint | States | Does not state |
|---|---|---|---|
| T1 | SHA-256 of `parte-1-declaracion.sig` | The Client signature existed at that time | Anything about position 2 |
| Td | SHA-256 of `contrato.pdf` | The PDF existed at that time; `openssl ts -verify -data` still works | Anything bilateral |
| Tf | SHA-256 of `contrato-manifiesto.json` | Both statements and signatures, T1, Td, certificates and CRLs existed at that time | That anybody read the contract |

The tokens must satisfy `T1 <= Td <= Tf` and `created_at <= genTime(T1)`.
For T1 and for Tf, `genTime + accuracy <= expires_at`: using
`genTime - accuracy` would hand the firm extra margin. No separate token
covers the counter-signature: Tf covers it. The real order is proven by the
hash chain, not by the authority: the position 2 statement declares under its
key which signature and which token it saw.

The authority is called outside any transaction, once per sign request and
without automatic retry. An unavailable authority or an invalid token leaves
no state (`contract_timestamp_unavailable`), and the person resubmits the same
bytes by explicit action. The token anchor must be the credential root, read
from the same configuration as the existing external signing path.

### The counter-signature and the Client binding

The Client cannot withdraw after signing: that would be a session event
without a signature, outside the package, a state that promises something the
evidence cannot show. The counter-signature therefore does not depend on the
Client binding staying alive. The Client may withdraw the binding after
signing; that only prevents future signatures with it and does not touch a
request already signed. The only thing that blocks the counter-signature is a
revocation of the Client certificate already published in the current CRL
(`contract_counterparty_revoked`), which goes through the CA operator and is
captured in the package. The limit the Client does control is the signed
`expires_at`. Before the Client signs, any certificate change of either party
requires a new request; after it, only a change of the firm signer does.

Rejection, decline, cancellation and expiry are application records; no
package refutes them.

### Evidence packages

Both profiles use fixed entry names only: they admit no free document name and
look for no sibling entries. `FIXED_NAMES` of the existing profiles does not
change, so no upload or individual export is newly rejected. No existing
profile contains `contrato-solicitud.bin`, and neither new profile contains
`certificado.pem`, so profile selection stays unambiguous. `INSTRUCCIONES.md`
is optional in every profile, never interpreted, and always exported by Qadra.
The admission size limits gain `contrato.pdf` with the document limit, and
`parte-1-crl.pem`, `parte-2-crl.pem`, `parte-1-vinculo-crl.der` and
`parte-2-vinculo-crl.der` with the CRL limit, checked after the existing
document-candidate rule so that no v1 or v2 package changes outcome.

**Full package, profile `BilateralContractV1`: 26 mandatory entries plus the
optional `INSTRUCCIONES.md` (27 of the 32-entry limit).**

- Common (8): `contrato.pdf`, `contrato.pdf.tsr`, `contrato-solicitud.bin`,
  `contrato-manifiesto.json`, `contrato-manifiesto.json.tsr`,
  `contrato-recibo.json`, `ca.pem`, `tsa-chain.pem`.
- Per party, for N in 1 and 2 (8 x 2): `parte-N-declaracion.bin`,
  `parte-N-declaracion.sig`, `parte-N-certificado.pem`, `parte-N-crl.pem`,
  `parte-N-vinculo.bin`, `parte-N-vinculo.sig`, `parte-N-vinculo-raiz.der`,
  `parte-N-vinculo-crl.der`.
- T1 (2): `parte-1-declaracion.sig.tsr`, `parte-1-tsa-chain.pem`.

The manifest covers every entry except itself, its token
`contrato-manifiesto.json.tsr`, `contrato-recibo.json` and
`INSTRUCCIONES.md`. The manifest is RFC 8785 JSON with `profile`
`qadra-evidence-manifest`, `profile_version` 2, `purpose`
`bilateral-contract-signing`, the algorithms of ADR 0085, the deployment,
case, request, document, version, PDF and request digests, `expires_at`,
`artifacts` as `{entry, party, role, sha256, size}` with `party` from 0 to 2,
and exactly two `signers` as `{id, position, side, account_id, account_role,
binding_id, binding_registered_at, operation_id, entries}`. `account_role` is
application data, not signed bytes. The version 1 manifest parser and
`manifest.signers_not_supported` are not touched. `contrato-recibo.json`
records each party's `accepted_at`, `completed_at`, the ceremony reference and
`provenance: application-record`; no token covers it and it decides nothing.

**Contract manifest members.** The top-level members are exactly
`profile`, `profile_version`, `purpose`, `deployment_id`, `case_id`,
`request_id`, `document_id`, `document_version` (at least 1),
`document_sha256`, `request_sha256` (of `contrato-solicitud.bin`),
`expires_at` (the request's, non-negative integer Unix seconds), `algorithms`,
`artifacts` and `signers`. Identifiers are lowercase hyphenated UUIDs and
digests lowercase hexadecimal. The artifacts are sorted by entry name in byte
order, each `(party, role)` pair appears once, and each pair names exactly
one entry:

| Party | Role | Entry |
|---|---|---|
| 0 | `document` | `contrato.pdf` |
| 0 | `document-timestamp` | `contrato.pdf.tsr` |
| 0 | `request` | `contrato-solicitud.bin` |
| 0 | `issuer-certificate` | `ca.pem` |
| 0 | `tsa-chain` | `tsa-chain.pem` |
| N | `statement` | `parte-N-declaracion.bin` |
| N | `statement-signature` | `parte-N-declaracion.sig` |
| N | `signer-certificate` | `parte-N-certificado.pem` |
| N | `issuer-crl` | `parte-N-crl.pem` |
| N | `binding-statement` | `parte-N-vinculo.bin` |
| N | `binding-signature` | `parte-N-vinculo.sig` |
| N | `binding-root` | `parte-N-vinculo-raiz.der` |
| N | `binding-crl` | `parte-N-vinculo-crl.der` |
| 1 | `statement-timestamp` | `parte-1-declaracion.sig.tsr` |
| 1 | `statement-tsa-chain` | `parte-1-tsa-chain.pem` |

N is 1 or 2, so a full manifest lists 23 artifacts. The two signers come in
position order: `id` is `signer-1` or `signer-2`, `position` 1 or 2, `side`
`client` or `firm`, and `account_role` `client` for position 1 and `owner` or
`litigator` for position 2. `account_id`, `binding_id` and `operation_id` are
the party's, `binding_registered_at` is non-negative integer Unix seconds, and
`entries` has exactly the members `statement`, `statement_signature`,
`signer_certificate`, `issuer_crl`, `binding_statement`, `binding_signature`,
`binding_root` and `binding_crl`, each naming that party's entry of the
matching role. A schema departure fails `manifest.contract_schema`.

**`contrato-recibo.json`** is RFC 8785 JSON with exactly these keys:
`provenance` (the string `application-record`), `request_id`,
`request_sha256`, `ceremony_reference` (1 to 80 characters),
`client_accepted_at`, `firm_accepted_at` and `completed_at` (non-negative
integer Unix seconds). Bytes that are not canonical RFC 8785 JSON in the ADR
0085 subset fail with the `json.*` codes; well-formed JSON with other members
or values fails `receipt.contract_schema`. Its values are reported as
application records and decide nothing.

**Partial package, profile `BilateralContractPartialV1`: 14 mandatory entries
plus the optional `INSTRUCCIONES.md`.** `contrato.pdf`,
`contrato-solicitud.bin`, `contrato-acuse-parcial.json`, `ca.pem`, the eight
`parte-1-` party entries, `parte-1-declaracion.sig.tsr` and
`parte-1-tsa-chain.pem`. It has no manifest: the position 1 statement already
binds request, PDF and both leaves. It is offered to the Client from T1 on,
in the same signing flow, and the Client keeps it if the request later closes
without completion.

**`contrato-acuse-parcial.json`** is RFC 8785 JSON in the ADR 0085 subset with
exactly these keys: `provenance` (the string `application-record`),
`request_id` (lowercase hyphenated UUID), `request_sha256` (lowercase
hexadecimal SHA-256 of `contrato-solicitud.bin`), `binding_registered_at`,
`accepted_at` and `exported_at` (non-negative integer Unix seconds) and
`ceremony_reference` (the declared sheet reference, 1 to 80 characters). No
token covers it and it decides no result. Bytes that are not canonical JSON in
that subset fail with the `json.*` codes; well-formed JSON with other members
or values fails `receipt.contract_partial_schema`. It never appears in a full package, so
removing any mandatory entry of a full package gives `profile.missing_entry`,
never a partial and never `checked`.

**Partial package judged at T1.** The partial package has no sealed manifest
that fixes when the position 1 binding was registered, so the binding is
judged at T1 against `parte-1-vinculo-crl.der`: the binding existed and was not
revoked at the time-stamped instant of the signature. In the full package the
registration instant comes from the manifest, covered by Tf. The partial
package always emits `contract.partial` as `Indeterminate`, so its overall
result is never `checked`; it is labelled as an acknowledgement that does not
establish a contract.

**Instants.** The Client is judged at T1 and the firm signer at Tf, through
the token interval, never through `accepted_at` or any receipt. Each party's
signature CRL is `parte-N-crl.pem` at its token. The authority CRL is
`parte-1-crl.pem` for T1 and `parte-2-crl.pem` for Td and Tf, so a CRL renewal
between the two signatures never leaves T1 without a covering list. The serial
of the position 1 certificate must not appear in `parte-2-crl.pem` with a
revocation instant up to Tf. A revocation after completion keeps the
historical result. A CRL never lists a revocation after its own `thisUpdate`,
so a package list that covers Tf cannot name a later one, and the
`crl.revoked_later` path of the counterparty check does not arise from
exported packages: a revocation after completion appears only in the
informative current status, as `crl.revoked` against a CRL served at
validation time, while the historical result stays checked. The fixtures of
`crates/evidence-verify/tests/contract-fixtures` exercise this and each rule
below; their README lists every variant ceremony and its expected result.
The check components stay the eight of ADR 0085; the document token component
becomes optional so that the partial package reports it as not applicable,
while v1 and v2 reports are unchanged.

**Checks.** Each party statement must name the request, the SHA-256 of
`contrato-solicitud.bin`, the PDF digest of the request and of `contrato.pdf`,
its own position, and the account and certificate leaf the request fixes for
that position, which must be the digest of `parte-N-certificado.pem`; any
departure fails `contract.statement_mismatch` or `contract.party_mismatch`.
Each party's `DOCBIND1` binding must describe the same deployment, account,
binding, root and leaf as its statement, with counters that do not decrease.
Both statements and both binding roots must name `ca.pem`
(`contract.root_mismatch`). The serial of the position 1 certificate counts as
revoked in `parte-2-crl.pem` when its revocation instant is not after the end
of the Tf interval (`genTime` plus accuracy), the same bound every other
revocation check uses. In the full package each `binding_registered_at` of
the manifest must not be after the end of its party's token interval. The
components are: Td for the document token; both statement signatures and T1
for the document signature; the request, statements and bindings for binding
and consent; each party's chain and the three time-stamping chains for
certificates; each party's binding and signature CRL, each token's
time-stamping CRL and the counterparty rule for revocation; the manifest, its
coverage and Tf for the manifest; and the hash chain, the token order, the
validity window, the receipts and `contract.partial` for the context.

### Finding codes

`CTRTREQ1` structural failures emit `contract.request_schema`. `CTRTSIG1`
structural failures emit the existing `statement.*` codes of the shared prefix
plus `statement.position` for a position byte other than 1 or 2 and for a
position 2 statement with an all-zero chain digest. A position 1 statement
with a non-zero chain digest does not re-encode to its own bytes and fails
`statement.not_canonical`, as the domain decoder also treats it. Bytes of
`contrato-manifiesto.json`, `contrato-recibo.json` or
`contrato-acuse-parcial.json` that are not canonical JSON in the ADR 0085
subset fail with the `json.*` codes before any schema check. The
remaining new codes are `contract.statement_mismatch`,
`contract.party_mismatch`, `contract.same_account`, `contract.same_key`,
`contract.root_mismatch`, `contract.chain_mismatch`, `contract.token_order`,
`contract.before_issue`, `contract.after_expiry`,
`contract.counterparty_revoked`, `contract.partial`,
`manifest.contract_schema`, `receipt.contract_schema` and
`receipt.contract_partial_schema`. Each has a plain Spanish message in
`web/apps/qadra/src/lib/evidence/messages-contract.mjs`.

### Audit actions and resources

Identifiers in resources are lowercase hyphenated UUIDs and `{n}` is the
decimal contract event revision. The triggers compare these strings
literally.

| Action | Resource |
|---|---|
| `contract.created` | `contract:{contract_id}:document:{document_id}` |
| `contract.abandoned` | `contract:{contract_id}:revision:{n}` |
| `contract.requested`, `contract.client_signed`, `contract.rejected`, `contract.completed`, `contract.declined`, `contract.cancelled`, `contract.expired` | `contract-request:{request_id}:revision:{n}` |
| `contract.content_read` | `contract-request:{request_id}:content` |
| `contract.evidence_exported` | `contract-request:{request_id}:evidence:{stage}`, with `stage` `partial` or `final` |

Expiry is written by the technical actor `system:contract-expiry`. Creating
and abandoning require Owner or an assigned Litigator, abandoning requires a
reason and is allowed only from `borrador`; cancelling requires a reason and
is allowed to the requester or an Owner.

### HTTP errors

| HTTP | Code | Situation |
|---|---|---|
| 404 | `contract_not_found` | Missing, or no relation to the caller, without distinction |
| 409 | `contract_revision_conflict` | `expected_revision` differs |
| 409 | `contract_exists` | The document series already has a contract |
| 409 | `contract_request_open` | An open request already exists |
| 409 | `contract_request_closed` | The request is closed |
| 409 | `contract_out_of_turn` | Signature out of order |
| 409 | `contract_expired` | Validity exhausted |
| 409 | `contract_version_superseded` | A later document version exists |
| 409 | `contract_version_frozen` | Seal or individual signature on a contracted version |
| 409 | `contract_counterparty_revoked` | Client certificate revoked in the current CRL before the counter-signature |
| 409 | `contract_not_complete` | Final package requested without completion |
| 422 | `contract_party_ineligible` | With `blockers` among `client_not_ready`, `firm_signer_not_ready`, `document_not_ready`; the first never distinguishes a missing, ineligible or unbound account |
| 503 | `contract_timestamp_unavailable` | The authority failed; no state was written |
| 400 | `invalid_contract_request` | A path value, body member, base64 value or outcome is malformed |

The `contract_party_ineligible` envelope carries the blocker codes as an
array member `blockers` of its `error` object, next to `code` and `message`.
The exact content route answers an explicit HEAD with 405, like the document
content route, so a HEAD request never decrypts the version or records a
`contract.content_read` event.

The individual signing codes `signing_account_changed`,
`signing_trust_changed`, `signing_conflict`, `signing_binding_inactive` and
`signing_credential_rejected` are reused with their existing meaning. Every
mutation carries `operation_id` and `expected_revision`; an uncertain response
is reconciled through the exact operation receipt, and an identical
resubmission returns the original receipt while different bytes give
`signing_conflict`. There is no route that looks accounts up by e-mail.

### Client onboarding

The Client private key never reaches the server.

1. Only an Owner invites an account with role `client`, through the existing
   invitation workflow (24-hour validity, resend). The Client chooses their
   password and confirms TOTP. `migrations/0051_client_invitations.sql`
   widens the invitation role constraint of
   `migrations/0046_member_invitations.sql` when it still has its original
   form, without touching stored invitations, and startup requires the
   widened constraint. It is registered with the invitation family and is a
   separate file from the membership guard, because the two families migrate
   at different points. An invitation never creates an Owner.
2. The Client generates an encrypted key and a CSR locally in "Mi firma", or
   uses OpenSSL, and sees the SHA-256 of the SPKI computed locally.
3. The Client hands over the public CSR through an external channel and reads
   the SPKI fingerprint in person or by phone. The CA operator compares it in
   the issuance preview, issues the existing `v3_internal_declaration` profile
   and records the sheet reference. Issuance creates no account or binding.
4. The Client uploads the certificate and signs `DOCBIND1`, creating a
   binding as for individual signing.
5. Client and firm sign by hand two copies of the fingerprint sheet: sheet
   reference, account e-mail, SPKI SHA-256 read from the Client's device, root
   fingerprint, date and handwritten signatures. It is anchored on the SPKI
   because the SPKI exists before issuance and survives re-issuing the same
   key. It is the only measure that ties person and key outside the firm's
   technical control; it does not establish civil identity. The sheet
   reference is required to create a request.

A document-signing binding (ADR 0074) may therefore be held by every party of
a contract: the binding service, the signing account lookup and the storage
guard admit an active Owner, Litigator or Client, and still reject a
Paralegal. `migrations/0050_signing_binding_parties.sql` replaces the two
binding functions of `migrations/0044_document_signing_bindings.sql` with
`CREATE OR REPLACE` and changes no table, column, trigger or row; the startup
inventory compares each function with its last registered definition.
Individual signing keeps requiring an active Litigator who is a case member,
in its service and in the receipt guard of
`migrations/0045_individual_document_signatures.sql`, so an Owner or Client
binding never produces an individual signature.

Only an account created by an accepted `client` invitation is eligible. An
account created directly by an Owner is not, because the Owner chose its
initial password and its TOTP secret was generated on the server. The
certificate carries the C and O of the root by CA policy; that is not an
affiliation. Lost keys are not recoverable: before signing, the Client
withdraws the binding, asks for revocation and repeats onboarding; after
signing, the signature stays and is judged at its own instant.

### Why the system no longer generates the Client key

If the server generates or uses the Client key, the firm can sign for its
counterparty, and the Client can rightly deny every signature, destroying the
very evidence the workflow exists to produce. The Client therefore generates
and holds the key, exactly as Litigators already do under ADR 0074 and
ADR 0079. The accepted costs are a key held in a file, a CSR delivered through
an external channel and an unrecoverable loss.

### Access

`Role::allows` gains `ManageContract` (Owner, Litigator) and `ReadContract`
(Owner, Litigator, Paralegal). `Role::allows(Client)` stays false for every
`Permission`: the Client role keeps zero permissions. A party's access is a
relation, checked on every operation by the contract service: the designated
Client and the designated firm signer of a request. `DocumentAction`, the
document `authorize` and the case scope checks are not changed.

| Action | Owner | Assigned Litigator | Assigned Paralegal | Designated firm signer | Designated Client | Any other account |
|---|---|---|---|---|---|---|
| Create, abandon, request | Yes | Yes | No | By role | No | No |
| Read state and events | Yes | Yes | Read | Yes | Own request only | 404 |
| Read the exact version through the request route | No (document route) | No (document route) | No | While signing | While open, or once signed | 404 |
| Sign | Only if designated | Only if designated | No | Position 2 | Position 1 | 404 |
| Reject | No | No | No | No | Before signing | 404 |
| Decline | Only if designated | Only if designated | No | After the Client signs | No | 404 |
| Cancel | Yes | Only the requester | No | No | No | 404 |
| Partial package | Yes | Yes | Yes | Yes | Once signed, also after an unsuccessful close | 404 |
| Full package | Yes | Yes | Yes | Yes | Yes, their own | 404 |

Staff package reads additionally require `ExportEvidence` and case scope. A
Litigator signer who loses case membership cannot sign. Nobody signs for
another account: the storage trigger compares the signer. An Owner signs only
with their own `document-signing` certificate, distinct from the `partner`
access certificate.

A Client account is never a case member. The case service, a storage trigger
on case memberships and the member selectors all reject or omit accounts with
role `client`. Assigning a Client answers `422 invalid_input` and writes
nothing. Because a role change would otherwise turn an existing member into a
Client member, the member service rejects changing the role of an account that
holds an assignment to `client` with `400 invalid_input`. The trigger guards
only writes to case memberships: every runtime path that could create a Client
member is closed by these checks, and a direct SQL role change is tampering
that the startup inventory and the migration detect. The guard lives in
`migrations/0051_client_memberships.sql`, registered with the member schema
family so that its startup inventory checks the function, the trigger and the
absence of any Client membership. The migration
refuses to run while a Client membership exists and never deletes one:
whether to keep or remove such an assignment is a decision for the firm, taken
through the audited member workflow before migrating. Client access is limited to a restricted Qadra space for the
Client's own contracts: no messaging, no case browsing and no general document
access. Any other read answers 404.

### Storage

Contract storage is one schema family, `crates/infrastructure/src/contract_schema`,
registered after the invitation family because the request trigger reads
accepted Client invitations. Its three migrations are split only to keep each
file under the 400-line limit and run in this order on every administrative
connection, idempotently: `migrations/0052_contracts.sql` (tables and the two
statement builders), `migrations/0052_contract_events.sql` (lock, preservation,
event and request registration) and `migrations/0052_contract_signatures.sql`
(signatures, completion, pairing, notifications, version guard and every
trigger). No table holds a state column.

- `contracts`: the root, one per document series (`document_id` unique), with a
  foreign key to `document_series(id,case_id)` and the captured creator.
- `contract_signature_requests`: every `CTRTREQ1` field, both binding
  identities (foreign keys to `document_signing_binding_identity`), the
  optional Client subject revision (which must belong to the same case), the
  `ceremony_reference` of 1 to 80 characters, the 240 bytes and their digest,
  `created_at_seconds`, `created_at_nanoseconds` and `expires_at_seconds`,
  with a validity of 1 to 30 days.
- `contract_events`: primary key `(contract_id, revision)`, unique
  `operation_id` and `audit_sequence`, the only foreign key to `audit_events`.
  An `expired` event has no actor account and the actor label
  `system:contract-expiry`; every other event names an account whose e-mail is
  its label. Abandoning and cancelling require a reason of 1 to 500
  characters; rejecting and declining admit one.
- `contract_party_signatures`: primary key `(request_id, position)`, the 303
  bytes, the 384-byte signature, the binding and trust revision used, and, in
  position 1 only, T1 with its time-stamping chain. Its `operation_id` is a
  foreign key to the event that the signature completes.
- `contract_completions`: Td, the manifest of at most 64 KiB with its digest,
  Tf and the time-stamping chain. Each party CRL is the CRL of the trust
  revision its signature names, so no CRL is stored twice.
- `contract_notifications`: the only mutable table. Each row names its request
  and its exact event, a recipient, the event kind, sender, recipient address,
  `subject` (1 to 200 characters) and `message_text` (1 to 4096 characters),
  and delivery columns as invitation deliveries have. A `requested` notice goes
  to the Client; `client_signed`, `completed` and `expired` notices to the
  Client, the firm signer and the requester; `rejected` to the requester and
  the firm signer; `declined` to the Client and the requester; `cancelled` to
  both parties. Only the delivery columns change, never downward in generation.

Every write takes the advisory lock 280603412820 under READ COMMITTED, after
the audited mutation lock. Within one transaction the audit row comes first;
the root precedes its `created` event and a request its `requested` event; a
`client_signed` or `completed` event precedes its signature; the completion
follows the position 2 signature. The deferred trigger function
`contract_completion_pair` refuses at commit a root without revision 1, a
request without its `requested` event, a `client_signed` event without the
position 1 signature, and a `completed` event without the position 2 signature
and the completion.

`contract_event_register` requires the previous revision plus one
(SQLSTATE 40001, `contract revision conflict`), a non-decreasing instant and
audit sequence, the exact audit row, the event graph of the state table above,
and the designated actor of each event. The audit chain digest is computed by
the application, so no trigger can write an event. The expired-closing rule is
therefore enforced, not performed, by storage: while the previous request is
nominally open, a new request or an `abandoned` event is refused with
`contract request is open` before `expires_at` and with
`contract request must expire first` after it, and the adapter writes the
audited `expired` event of the exhausted request first. Abandoning follows the
same rule, so no `expired` event can ever follow a terminal one.

`contract_request_register` checks the open case, the latest version of the
series with `documents.evidence IS NULL` read `FOR SHARE`, the subject's case,
an active Client with role `client` created by an accepted `client`
invitation, an active Owner or member Litigator as firm signer and as
requester, both bindings unwithdrawn, registered no later than the request and
under the current root, and rebuilds the 240 bytes with `contract_request_bytes`.
`contract_signature_register` rebuilds the 303 bytes with `contract_party_bytes`
(the position 1 chain digests are zeros; position 2 digests the stored position
1 signature and T1) and requires the designated signer, binding and leaf, the
event of its turn, `accepted_at < expires_at`, the active account with
unchanged counters and role, membership for a Litigator in position 2, the
open case, the latest version, the signer's binding unwithdrawn, the current
trust revision and the same root as position 1. It does not consult the Client
binding in position 2.

The registration triggers compare the record's own instants, which come from
the application clock. `contract_version_guard`, `BEFORE UPDATE OF evidence ON
documents`, has no such instant and reads the database clock: it refuses
evidence (`contract version is frozen`) for a version whose request is
completed, or has no closing event and has not reached `expires_at`. An
exhausted request therefore frees its version before its `expired` event is
written. A request waiting on an uncommitted seal, or a seal waiting on an
uncommitted request, re-reads the committed row and is refused.

Authorization failures use SQLSTATE 42501, revision conflicts 40001 and every
other refusal 23514; each message is stable so the adapters, which check the
same conditions before writing, can map a lost race to the HTTP codes above.
The runtime role may read and insert the five append-only tables, read,
insert and update only the delivery columns of notifications, and execute the
two statement builders; it cannot execute any trigger function. The only new
reference to audit rows raises the audit foreign-key triggers from 26 to 28.

The startup inventory checks, in SQL, every CHECK predicate, the created event
of each root, contiguous revisions, the event graph, each event's audit row,
each request's `requested` event and rebuilt bytes, each signature's event,
designation and rebuilt bytes, the pairs above, and that a completed version
holds no evidence. It then decodes every statement with the domain codecs,
verifies each party signature with the trust revision it names at its
`checked_at`, verifies T1, Td and Tf over their imprints under the admitted
policy, and parses each manifest to compare its identities, signers and every
artifact whose bytes are stored. Token instants against `created_at` and
`expires_at` are judged by the signing service and the validator, and the
certificates and CRLs encoded for a package are checked by the exporter.

### Application service

`crates/application/src/contracts` holds the use cases, their ports and the
pure rules above, so the store only supplies facts and rechecks them under
its locks. A store read returns the whole stored contract (root, events,
requests with both live-at-request binding receipts, signatures with the
root and CRL of the trust revision they name, and the completion) plus
whether the caller has staff scope on the case, the latest version of the
series and whether the case is closed. The service derives every state and
decides every access from those facts; the derivation is the single
definition of an open request given above.

These details are fixed by the service:

- An account that can read a contract but is not the actor of an action
  (a Paralegal who tries to manage, a staff member who is not the designated
  signer, a party who tries the other party's action) receives
  `PermissionDenied`; an account with no relation at all receives
  `contract_not_found`.
- The designated firm signer may read the exact version through the request
  route while the request is open, also before the Client signs, so the
  firm can review what the Client is asked to accept.
- A request is created with the application clock: `created_at` is the
  instant of the command in whole seconds and `expires_at` adds the
  validity in days. Party eligibility is resolved inside the command and
  reported only as the three blockers.
- Signing builds the expected statement from the current account counters,
  the current trust revision and the request's designated binding, which
  must still be the caller's live binding. A submitted statement whose
  account counters differ answers `signing_account_changed`, whose
  deployment, root or trust revision differ `signing_trust_changed`, and
  any other difference `signing_conflict`.
- The position 2 manifest is encoded from the exact bytes the export
  writes: certificates and CRLs as canonical PEM of their stored DER, `ca.pem`
  from the root of the position 2 trust revision, `tsa-chain.pem` from the
  anchor of Td and Tf, and `contrato.pdf` from the decrypted, digest-checked
  content. Td and Tf must use the same anchor. The export rebuilds the
  covered entries and refuses to write a package whose artifacts differ from
  the stored manifest.
- Every T1, Td and Tf failure that is not a configuration error answers
  `contract_timestamp_unavailable`, including a token whose `genTime`
  precedes `created_at` or, for Td and Tf, its predecessor token. A token
  whose `genTime` plus accuracy passes `expires_at` answers
  `contract_expired`.
- Notices carry a plain ASCII subject and text that name the contract
  title, the request and its expiry, and link to `public_url#contracts`
  when a public address is configured; they never carry a token, signature
  or certificate.
- The expiry closer runs with no actor, writes one `expired` event per
  exhausted unclosed request, at most 20 per cycle, and skips a request
  whose revision another writer changed in between.
- Creating a second contract for a document series answers 409
  `contract_exists`; the series identifies its contract.
- Document lists, details and version histories carry the derived
  `contract_state` of the series (`None` without a contract), computed in SQL
  with the instant of the read. The `sealed=false` filter omits a current
  version that a completed request names or that an unclosed request names
  before its `expires_at`, because no seal can be written for it. The seal
  and individual signing adapters check the same freeze with the database
  clock, the instant the storage guard reads, and answer
  `contract_version_frozen` before writing.
- `PostgresContractStore` returns a contract to a party without staff scope
  with only the requests that designate it, and blanks the actor and reason
  of every event of another request, while keeping every revision so the
  party can send the right `expected_revision`. The store finds the exact
  replay of an operation inside the write transaction, after the audited
  mutation lock and the contract lock, before rechecking the author and the
  expected revision, and maps each trigger refusal to its contract error.

## Consequences

What the evidence proves: the holder of each bound key accepted that exact
version, against that counterparty and that request; position 2 signed over
what position 1 signed, in that order; nobody changed content, counterparty,
certificate or validity without breaking signatures; according to T1 and Tf
both signatures happened before `expires_at`, under the reservation that the
authority belongs to the firm; and without the second signature there is no
complete contract. It does not prove civil identity, capacity,
representation, actual reading, which instant is true, that a declared name
is true, a rejection, cancellation or expiry, or legal validity.

Inherent limits, declared here and in the academic report:

- **(a) One root for everything.** CA, seal and time-stamping authority belong
  to the firm. Time is the firm's declaration, and the firm can backdate Tf
  inside the validity. Holding the Client signature as a blank cheque is
  therefore mitigated only against an honest server: the signed `expires_at`
  checked against Tf does not stop a firm that controls the authority and
  dates Tf before `expires_at`.
- **(b) Fabricating a person.** Identity is declared and the common name is
  informative. A firm can create a Client with a mailbox of its own. The
  fingerprint sheet and a separate CA operator detect this; they do not
  prevent it.
- **(c) Fair exchange.** Without a third party it is impossible: between T1
  and Tf the firm holds an option until `expires_at`.
- **(d) Unprovable closings.** Rejection, cancellation and expiry are not in
  the package.
- **(e) Unpublished revocation.** A revocation is not visible until the next
  CRL.
- **(f) The firm serves the page.** The page could capture key and password
  while in use. This is not custody, but it is the real path; OpenSSL reduces
  it.
- **(g) What is seen depends on the viewer.** Admission is structural and does
  not filter active content.
- **(h) Theft claim.** The Client can claim theft of key and password.
- **(i) Lost access.** A Client who did not download and is deactivated loses
  access. A downloaded package does not depend on Qadra.

The strongest direction is against the firm: a Client holding the package can
show the firm's signature under the firm's own root. Against the Client, the
evidence was produced by the other party, with its own CA, authority, database
and JavaScript; its weight depends on the onboarding ceremony and on what each
party keeps outside the system. Without a fingerprint sheet the Client can
claim fabrication.

Distinct accounts, leaves and SPKIs are checked, but they do not stop someone
who generates two keys. N-party contracts are not supported: three parties
would exceed the 32-entry archive limit and the fixed order does not
generalize. Verification with OpenSSL is a two-step procedure documented in
`INSTRUCCIONES.md`, which reproduces every signature, digest and token without
Qadra.

The individual signing tables, their storage functions and their inventories
are not changed, apart from the two binding functions that admit every
contract party, and every v1 and v2 package keeps its verdict. New storage
adds append-only contract tables, two audit references and a version guard
that blocks seals and individual signatures on contracted versions. The
dashboard pending-contract indicator is redefined over contracts in
`borrador` or `en_firma` instead of the free-text document type.
