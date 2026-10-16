# ADR 0079: Browser-local keys and exact-byte signatures

## Status

Accepted and implemented. Browser-local cryptography is integrated through
PR110 and available in Qadra v0.1.12. Acceptance boundaries are recorded in the
[verification report](../verification-report.md). The user workflow and supported formats are in
[browser-local cryptography](../browser-local-cryptography.md).

## Context

Owner registration, Owner certificate login and individual document signing
already admit public certificates and detached RSA signatures. Their server
contracts capture exact bytes, identity and current authority. Requiring an
external signing command for each capture makes those existing workflows
difficult to operate from the office interface.

The CA already accepts public CSRs and reviews the holder, issuance purpose
and issuer policy. Generating a holder key in the CA would change custody and
is not needed to prepare that request. Browser-local generation must keep the
existing CA review and separate account-binding operations.

[ADR 0002](0002-rsa-signing-crate-and-advisory.md) accepted the RSA crate's
private-operation timing advisory under an earlier local CLI threat model.
That rationale does not authorize a new browser signing surface. A worker
and WebAssembly do not remove the page's authority or establish a new security
boundary against compromised same-origin code.

## Decision

### Keep private operations in a cancelable local worker

Use the Rust `browser-crypto` crate, compiled to WebAssembly, from a dedicated
worker. Create a worker for one operation and terminate it on completion,
failure or cancellation. Invalidate late results when the account, prepared
bytes, certificate, selected version or authorization context changes.

Use `ring` for RSA private signing with SHA-256 and PKCS#1 v1.5, including CSR
signatures. Restrict `rsa` to key generation and key parsing/representation;
do not use its private signing API in this browser helper. This separates the
new signing path from the risk accepted by ADR 0002 without changing the
existing server implementation or declaring that advisory solved generally.

Generate RSA-3072 keys with exponent 65537. Obtain entropy from the browser's
cryptographic random source through the Rust browser target. Missing entropy
fails the operation; do not substitute timestamps, `Math.random` or a seed
chosen by the page's user.

### Export encrypted holder material and preserve interoperability

Export the private key only as PKCS#8 `ENCRYPTED PRIVATE KEY` PEM using PBES2,
PBKDF2-HMAC-SHA256 with 600000 iterations and AES-256-CBC. Generate independent
16-byte salt and IV values. This is the selected interoperable encrypted-key
format, not an authenticated-encryption or hardware-custody claim.

Import the same PBES2/PBKDF2-HMAC-SHA256/AES-256-CBC profile with iteration counts
from 1 through 2000000, preserving existing OpenSSL keys without rewriting
them. New generation always uses 600000. Reject unsupported key containers,
algorithms and oversized inputs before expensive processing. After decryption,
parse and validate the RSA key and its relation to the requested public key.

Limit encrypted PEM to 64 KiB, public certificate PEM or DER to 16 KiB, CSR PEM
to 16 KiB and password to 1024 UTF-8 bytes. A new password requires at least
12 Unicode characters and exact confirmation; do not normalize or trim it.
CSR names and organizations require 1 through 128 UTF-8 bytes without control
characters. The country is exactly two uppercase ASCII letters. These declared
fields do not establish civil identity or override the issuer's subject policy.

### Require a usable saved key before delivering its CSR

Offer generation in the authenticated Owner certificate and Litigator signing
views. Keep the two intended certificate purposes distinct. The generated
request does not select issuer authority or create an account or binding.

Require the user to download the encrypted key, select the saved file again
and provide its password. Decrypt locally and compare its public key with the
CSR before enabling CSR download. A click on a download button alone does not
prove that a usable backup exists. Deliver only the public CSR to the existing
CA operator for review and issuance.

### Reuse exact prepared operations

For Owner registration, Owner login, documentary binding, exact document
content and its separate consent, sign only the bytes already selected and
reviewed by the corresponding workflow. Check that the imported key matches
the expected public certificate and fingerprint. Return a detached 384-byte
signature to the existing form; signing locally does not submit or confirm
the server operation, bypass MFA or establish certificate trust.

Keep external OpenSSL signing and detached-signature import available. Do not
add arbitrary file signing, participant-declaration helpers, new certificate
purposes, automatic CA approval or a ZIP verifier in this delivery.

## Consequences

The server continues to receive only public certificates, CSRs where applicable,
prepared data and detached signatures. Private keys and their passwords are
used only by the browser-local helper; they are not placed in API payloads,
logs, persistent browser storage or recoverable form drafts. Clear fields and
references and terminate the worker on completion and context loss.

These lifetime controls do not guarantee physical erasure of JavaScript strings,
browser buffers, process memory, swap or crash dumps. An active page, compromised
application bundle, XSS, extension or compromised device can access material
while it is used. HTTPS, the delivered application's integrity and the user's
device remain part of the trust boundary. WebAssembly and a worker provide
implementation and execution controls, not isolation from that page authority.

The holder must preserve the encrypted file and password independently. The
application cannot recover either. Existing server authority, revocation,
certificate purpose, canonical-byte checks and operation receipts remain
decisive. Compatibility, cancellation, lifecycle and network-payload checks
require their own reproduced evidence before this implementation is accepted.
