# ADR-0011: Local document workflow behind the HTTP adapter

- Status: Accepted
- Date: 2026-08-17

## Context

The cryptographic use cases are implemented and the HTTP adapter exposes a
health endpoint, but no application workflow connects document storage,
encryption, signing, timestamping, verification, evidence export, and audit.
The web crate cannot depend on infrastructure without reversing the workspace
dependency direction. The external timestamp provider is not sufficiently
stable or transparent to be a runtime prerequisite.

## Decision

Define an object-safe document workflow port in the application crate. The web
adapter receives that port through application state and exposes versioned HTTP
routes. A concrete application service orchestrates existing domain ports for
encryption, hashing, signatures, timestamps, certificate validation, archive
creation, clock access, and audit storage.

Store each document version as an encrypted vault plus metadata and sealed
evidence behind a document repository port. Provide a local JSON repository
adapter whose binary fields use base64 and whose writes are atomic. The
repository never stores plaintext document content. The key encryption key and
signing private key remain runtime secrets and are not part of repository
records.

The initial HTTP contract uses a raw request body for document bytes and the
`X-Document-Name` header for its archive-safe name. Mutation requests accept an
`X-Actor` header solely as an audit label; it is not authentication. The
contract exposes upload, seal, verify, evidence download, and audit-chain
verification. Request authentication and production database adapters remain
separate increments.

The concrete runtime uses `LocalOpensslTsa`. Remote timestamp providers remain
optional adapters and are not selected implicitly.

## Consequences

- Axum remains independent of OpenSSL, files, private keys, and provider SDKs.
- The complete document path can run and be tested without network access.
- Stored document bodies are encrypted and bound to document identity and
  version through authenticated additional data.
- Evidence material is captured when the document is sealed, so later
  certificate or CRL changes do not silently rewrite historical packages.
- The local JSON repository is suitable for a reproducible prototype, not for
  multi-node production deployment.
- Document records and audit events live in separate files and do not share a
  transaction. A failure while appending the audit event can therefore leave a
  successful state change that the HTTP request reports as failed.
- The workflow serializes operations inside one process. Multiple server
  processes over the same directory and synchronous OpenSSL or file work on
  async request threads are outside this prototype contract.
- `X-Actor` must be replaced by authenticated identity before the HTTP surface
  is exposed outside a controlled local environment.
