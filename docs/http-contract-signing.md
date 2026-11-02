# Bilateral contract signing HTTP contract

All paths below start with `/api/v1`. Requests use the existing bearer session
and the shared HTTP admission and blocking-work limits. Responses have
`Cache-Control: no-store`. JSON bodies are objects of at most 16 KiB with
exactly the documented members; unknown members, arrays in place of objects,
query strings and bodies on GET requests are rejected. Identifiers are non-nil
lowercase hyphenated UUIDs and binary values canonical, padded standard
Base64. No endpoint accepts a private key, password or certificate container,
and no route looks an account up by e-mail: the request route resolves the
exact `client_email` inside its own command. The rules, statement layouts and
package profiles are fixed in
[ADR 0086](adr/0086-bilateral-contract-signing.md).

Every party binds its own document-signing certificate first through the
[individual signing binding routes](http-individual-signing.md), which admit
an Owner, a Litigator or a Client. Only an account created by an accepted
`client` [invitation](member-invitations-api.md) can be the Client of a
request.

## Case routes

These routes require staff scope on the case: an Owner, or an assigned
Litigator for every operation and an assigned Paralegal for reads. Any other
account, including every Client, receives `404 contract_not_found`; a
Paralegal who tries a mutation receives `403 permission_denied`.

| # | Method | Path | Body | Success |
| --- | --- | --- | --- | --- |
| 1 | GET | `/cases/{case}/contracts` | none | 200, contracts and eligible firm signers, at most 50 each |
| 2 | POST | `/cases/{case}/contracts` | `{"operation_id","document_id","title"}` | 201, receipt |
| 3 | GET | `/cases/{case}/contracts/{contract}` | none | 200, contract |
| 4 | POST | `/cases/{case}/contracts/{contract}/requests` | see below | 201, receipt |
| 5 | POST | `/cases/{case}/contracts/{contract}/abandon` | `{"operation_id","expected_revision","reason"}` | 200, receipt |
| 14 | GET | `/cases/{case}/contracts/operations/{operation}` | none | 200, receipt of a create or abandon |

The title has 1 to 160 characters and a reason 1 to 500, without control
characters. The server chooses the contract identifier, so an uncertain
creation is reconciled only through route 14. Route 4 takes:

```json
{"request_id":"uuid","operation_id":"uuid","expected_revision":1,"version":1,
"client_email":"cliente@example.test","firm_signer_id":"uuid","ttl_days":7,
"ceremony_reference":"HF-0001","subject":{"subject_id":"uuid","revision":"3"}}
```

`subject` is optional and its revision a decimal string. `ttl_days` is 1 to
30 and `ceremony_reference`, the reference of the signed fingerprint sheet, 1
to 80 characters. The firm signer is an active Owner or a Litigator member of
the case; route 1 lists the eligible ones.

A contract has this form; `state` is `borrador`, `en_firma`, `firmado` or
`abandonado`, derived at `derived_at_unix`:

```json
{"contract_id":"uuid","case_id":"uuid","document_id":"uuid","title":"...",
"created_by":"uuid","created_by_email":"...","created_at":"RFC3339",
"state":"en_firma","revision":2,"derived_at_unix":1793000000,
"events":[{"revision":2,"kind":"requested","request_id":"uuid","actor_id":"uuid",
"actor_label":"...","reason":null,"at":"RFC3339","operation_id":"uuid"}],
"requests":[<request summary>]}
```

A request summary carries `request_id`, `status` (`awaiting_client`,
`awaiting_firm`, `completed`, `rejected`, `declined`, `cancelled` or
`expired`), `contract_id`, `document_id`, `version`, `document_digest`,
`client_id`, `firm_signer_id`, `requester_id`, `requester_email`,
`firm_signer_role`, both certificate fingerprints, `created_at_unix`,
`expires_at_unix`, `created_at`, `ceremony_reference`, `subject` and the
instants `client_signed_at`, `firm_signed_at` and `completed_at`, each null
until it happens. A request whose validity is exhausted reads as `expired`
before its `expired` event is written.

## Request routes

These routes are reached by relation: the designated Client and firm signer
of the request, and staff with scope on its case. Any other account receives
`404 contract_not_found`. A related account that is not the actor of an
action receives `403 permission_denied`.

| # | Method | Path | Body | Success |
| --- | --- | --- | --- | --- |
| 6 | GET | `/contract-requests` | none | 200, `{"requests":[...]}`, the caller's own requests |
| 7 | GET | `/contract-requests/{request}` | none | 200, request detail |
| 8 | GET | `/contract-requests/{request}/content` | none | 200, exact PDF |
| 9 | POST | `/contract-requests/{request}/prepare` | `{"operation_id"}` | 200, statement to sign; writes nothing |
| 10 | POST | `/contract-requests/{request}/sign` | `{"operation_id","expected_revision","statement_base64","signature_base64"}` | 200, receipt |
| 11 | POST | `/contract-requests/{request}/close` | `{"operation_id","expected_revision","outcome","reason"?}` | 200, receipt |
| 12 | GET | `/contract-requests/{request}/operations/{operation}` | none | 200, receipt |
| 13 | GET | `/contract-requests/{request}/evidence/{stage}` | none | 200, ZIP |

Route 6 items are request summaries with `case_id`, `title`,
`contract_state` and the contract `revision`. Route 7 adds
`statement_base64` (the 240 `CTRTREQ1` bytes), `revision`, `title`,
`case_id`, `contract_state`, `client_certificate` and `firm_certificate`
(`der_base64`, `fingerprint` and the public `summary`) and `signatures`.

Route 8 serves the exact contracted version as an `application/octet-stream`
attachment of at most 16 MiB with `X-Case-Id`, `X-Document-Id`,
`X-Document-Version` and `X-Document-Digest`, like the document content route,
and records `contract.content_read`. The Client reads it while the request is
open or once it has signed; the firm signer while the request is open. A HEAD
request answers 405 and reads nothing.

Route 9 returns the 303 `CTRTSIG1` bytes for the caller's position with their
decoded fields: `position`, `account_id`, `account_revision`,
`auth_generation` (decimal strings), `binding_id`, `deployment_id`,
`root_fingerprint`, `trust_revision`, `certificate_fingerprint`,
`operation_id`, `request_id`, `request_sha256`, `document_digest`,
`client_signature_sha256` and `client_timestamp_sha256` (null in position 1),
`statement_base64` and `revision`, the value to send as `expected_revision`.
The party signs the exact statement bytes with RSA-3072 PKCS#1 v1.5 and
SHA-256 outside the server, for example with
`openssl dgst -sha256 -sign private.key.pem -out statement.sig statement.bin`,
and submits the 384-byte signature through route 10. The Client signs first;
the firm signer signs after it. The time-stamping authority is called once
outside any transaction and never retried.

Route 11 takes `outcome` `reject` (the Client, before signing), `decline` (the
firm signer, after the Client signed) or `cancel` (the requester or an Owner,
with a mandatory `reason`). These closings are application records that no
package refutes.

A receipt, returned by every mutation and by routes 12 and 14, has this form:

```json
{"contract_id":"uuid","case_id":"uuid","event":<event>,
"request_statement_base64":null,"signature":null,"manifest_sha256":null}
```

`request_statement_base64` holds the 240 request bytes for `requested`,
`signature` the statement, signature, `accepted_at` and, in position 1,
`timestamp_base64` (T1) for `client_signed` and `completed`, and
`manifest_sha256` the digest of `contrato-manifiesto.json` for `completed`.

Route 13 takes `stage` `partial` or `final` and returns
`application/zip`. `partial` is the `BilateralContractPartialV1`
acknowledgement, available to the Client once it signed, also after an
unsuccessful close; it never establishes a contract. `final` is the
`BilateralContractV1` package of a completed request. Staff reads also need
`ExportEvidence`. Both include `INSTRUCCIONES.md` with the OpenSSL check and
record `contract.evidence_exported`. The final package of a request is the
same bytes on every export.

## Reconciliation

Every mutation carries `operation_id` and, except creation,
`expected_revision`. There is no automatic retry. After an uncertain answer,
read route 12, or route 14 for create and abandon: 200 is the original
receipt; 404 means nothing was committed and the same bytes may be resent by
explicit action. An identical resubmission returns the original receipt;
different bytes under the same operation answer `409 signing_conflict`.

## Errors

The envelope is `{"error":{"code":"...","message":"..."}}`.
`contract_party_ineligible` adds `"blockers"`, a list among
`client_not_ready`, `firm_signer_not_ready` and `document_not_ready`; the
first never distinguishes a missing, ineligible or unbound account.

| HTTP | Code | Situation |
| --- | --- | --- |
| 400 | `invalid_contract_request` | Malformed path value, member, Base64 value, outcome or stage |
| 400 | `invalid_json` | Not a single JSON object or wrong content type |
| 404 | `contract_not_found` | Missing, or no relation to the caller |
| 409 | `contract_revision_conflict` | `expected_revision` differs |
| 409 | `contract_exists` | The document series already has a contract |
| 409 | `contract_request_open` | An open request already exists |
| 409 | `contract_request_closed` | The request is closed |
| 409 | `contract_out_of_turn` | Signature out of order |
| 409 | `contract_expired` | Validity exhausted, also by the token instant |
| 409 | `contract_version_superseded` | A later document version exists |
| 409 | `contract_version_frozen` | Seal or individual signature on a contracted version |
| 409 | `contract_counterparty_revoked` | Client certificate revoked in the current CRL before the counter-signature |
| 409 | `contract_not_complete` | `final` without completion, or `partial` read by staff or the firm signer before the Client signed; the Client itself receives 403 then |
| 413 | `contract_body_too_large` | Body over 16 KiB |
| 422 | `contract_party_ineligible` | With `blockers` |
| 503 | `contract_timestamp_unavailable` | The authority failed; no state was written |

`signing_account_changed`, `signing_trust_changed`, `signing_conflict`,
`signing_binding_inactive` and `signing_credential_rejected` keep their
[individual signing](http-individual-signing.md) meaning. Authentication,
permission and closed-case errors keep their existing codes.

## Notices and expiry

The server always runs one supervised contract consumer. Every five seconds
it writes the `expired` event of at most 20 exhausted requests. Each event
writes plain notices (title, request and expiry, never a token or signature)
for its recipients. Notice e-mail is sent only with
`TT_CONTRACT_EMAIL_ENABLED=true`, which reuses `REPORT_EMAIL_FROM`,
`REPORT_PUBLIC_URL` and `RESEND_API_KEY`; the link is
`REPORT_PUBLIC_URL#contracts`, without token or attachment. Without it,
notices stay stored and parties see their requests through route 6. Nothing in
the signing flow waits for e-mail. See
[database operations](database-operations.md) for the variables.

## Reproducible acceptance

`scripts/api-contract-demo.sh` runs the whole workflow against disposable
PostgreSQL and Redis on Unix sockets, a disposable CA and the local TSA. The
server and the HTTP client run in a private network namespace with only
loopback and the e-mail provider host pinned to loopback, so invitations and
notices are enabled but never leave the machine; the demo opens the stored
invitation capability with the disposable runtime key in place of the e-mail.
Each party generates its RSA-3072 key and CSR with OpenSSL; the CA issues only
the public certificate. The demo binds both keys, creates a contract and a
request, signs in turn, validates the partial and final packages with
`crates/evidence-verify/examples/validate.rs` and the OpenSSL two-step check,
closes other requests by rejection, decline after the Client signed and
cancellation, abandons a contract, and repeats the reads after a dump and
restore. It needs the pinned media decoder of
[media decoder setup](media-decoder-setup.md), like every `serve` run. Expiry
is not reached in the demo, because the shortest validity is one day; the
closer is covered by application and runtime tests.
