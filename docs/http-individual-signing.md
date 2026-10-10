# Individual document signing HTTP contract

All paths below start with `/api/v1`. Requests use the existing bearer session.
Responses have `Cache-Control: no-store`. JSON objects reject unknown and
duplicate fields. Base64 is canonical, padded standard Base64. No endpoint
accepts a private key, password, PKCS12 container or a client-selected account.
Account revision and authentication generation are decimal strings.

## Public certificate binding

These operations require the current Litigator account.

| Method | Path | Body |
| --- | --- | --- |
| GET | `/identity/signing-certificates/current` | none |
| POST | `/identity/signing-certificates/{binding}/prepare` | `{"certificate_base64":"..."}` |
| POST | `/identity/signing-certificates/{binding}/register` | `{"statement_base64":"...","certificate_der_base64":"...","signature_base64":"..."}` |
| GET | `/identity/signing-certificates/{binding}` | none |
| POST | `/identity/signing-certificates/{binding}/withdraw` | empty body |

The client generates a nonzero UUID binding ID. Prepare accepts a public PEM
or DER certificate up to 16 KiB; register uses the normalized DER returned by
prepare. The possession signature is exactly 384 bytes over the SHA-256 digest
of the returned canonical statement. Prepare returns:

```json
{"binding_id":"uuid","account_id":"uuid","account_label":"email",
"policy":"internal_individual_signing_v1","statement_base64":"...",
"account_revision":"9","auth_generation":"4","deployment_id":"uuid",
"trust_revision":7,"root_fingerprint":"hex",
"certificate":{"der_base64":"...","fingerprint":"hex","summary":{
"subject":"...","issuer":"...","serial_hex":"...",
"not_before_unix":0,"not_after_unix":2000}}}
```

Register, get and withdraw return the same receipt shape. Current returns
`{"binding":null}` or `{"binding":<receipt>}`. Receipt:

```json
{"binding_id":"uuid","account_id":"uuid","policy":"internal_individual_signing_v1",
"statement_base64":"...","signature_base64":"...","account_revision":"9",
"auth_generation":"4","certificate":{"der_base64":"...","fingerprint":"hex",
"summary":{"subject":"...","issuer":"...","serial_hex":"...",
"not_before_unix":0,"not_after_unix":2000}},"deployment_id":"uuid",
"trust_revision":7,"root_fingerprint":"hex","registered_at":"RFC3339",
"withdrawn_at":null}
```

## Exact document version

The prefix is `/cases/{case}/documents/{document}/versions/{version}/individual-signing`.
The URL is authoritative: a consent naming another case, document or version
is rejected. Versions are positive integers. Signing requires Litigator access
to the active case; receipt recovery can read a closed case.

| Method | Suffix | Body |
| --- | --- | --- |
| POST | `/prepare` | `{"operation_id":"uuid","pdf_signature_base64":"..."}` |
| POST | `/submit` | `{"consent_base64":"...","pdf_signature_base64":"...","consent_signature_base64":"..."}` |
| GET | `/receipts/{operation}` | none |
| GET | `/verification` | none |

The client explicitly generates a new nonzero operation UUID, signs the exact
downloaded PDF digest locally, then prepares and reviews the canonical consent
before signing its digest locally. Both signatures are exactly 384 bytes.
Prepare returns:

```json
{"operation_id":"uuid","case_id":"uuid","document_id":"uuid","version":1,
"name":"selected.pdf","document_digest":"hex","pdf_signature_digest":"hex",
"account_id":"uuid","binding_id":"uuid","certificate_fingerprint":"hex",
"trust_revision":7,"account_revision":"9","auth_generation":"4",
"consent_base64":"...","policy":"internal_individual_signing_v1"}
```

Submit and receipt recovery return:

```json
{"status":"accepted","operation_id":"uuid","case_id":"uuid","document_id":"uuid",
"version":1,"document_digest":"hex","pdf_signature_digest":"hex","account_id":"uuid",
"binding_id":"uuid","certificate_fingerprint":"hex","trust_revision":7,
"account_revision":"9","auth_generation":"4","consent_base64":"...",
"policy":"internal_individual_signing_v1","accepted_at":"RFC3339"}
```

Exact retry preserves the original receipt and timestamp. A changed consent or
either changed signature conflicts. After a lost response the client uses the
same operation and receipt URL, without generating a new operation.

Verification allows the existing per-case VerifyDocument roles. It returns
`{"kind":"individual"|"legacy_server","historical":<existing verification report>,
"certificate_current":{"status":"valid"|"rejected"|"trust_unavailable"|"not_individual",
"reason":null|"credential_failure_code","checked_at_unix":123},
"binding_withdrawn_at":null|"RFC3339","signing":null|<accepted receipt>}`.
The historical report uses captured evidence and the acceptance time for
individual signatures. Current certificate rejection or binding withdrawal is
reported separately and does not rewrite historical validity. Legacy server
evidence is never represented as a personal signature.

## Errors

Existing error envelope: `{"error":{"code":"...","message":"..."}}`.
Malformed encoding or reference: 400 `invalid_signing_request`; malformed JSON
objects or content type: 400 `invalid_json`; proof/certificate rejection:
422 `signing_credential_rejected`; changed account: 409 `signing_account_changed`;
changed trust: 409 `signing_trust_changed`; operation conflict: 409
`signing_conflict`; inactive binding: 409 `signing_binding_inactive`; missing
receipt or binding: 404 `signing_not_found`; trust unavailable: 503
`signing_trust_unavailable`. Authentication and permission errors retain the
existing identity codes. Internal incoherence returns the existing generic 500
without adapter details. Body limit is 32 KiB; oversized requests return 413.

## Reproducible acceptance

`scripts/api-demo.sh` includes individual signing capture, revocation and
restore through `scripts/api-individual-signing-demo.sh`. It uses the existing
disposable PostgreSQL, Redis, CA and local TSA. The holder generates an RSA-3072
key and CSR outside the CA; `pki issue-request --csr PATH --name NAME` issues
only its public certificate. Acceptance signs the exact downloaded PDF and
consent with the holder key, then verifies the three detached proofs and the
RFC 3161 token from the exported ZIP using OpenSSL.

The live browser family `individual-signing-real` uses the `individualSigning`
fixture in partition 3. `scripts/web-demo.sh` provisions distinct desktop and
mobile Litigator accounts and holder-generated CSR certificates. Private keys
remain in the disposable client fixture for local signing; neither the browser
form nor the API receives them.
