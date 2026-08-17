# 0009. Local timestamp authority for the current evidence

## Status

Accepted

## Context

The prototype needs an RFC 3161 timestamp authority to demonstrate the
complete flow: create a request, issue a DER token, bind its message imprint
to the document digest, verify the token, and export independently verifiable
evidence.

Cincel remains represented by an adapter and a local HTTP stub. The project
registration could be completed, but the sandbox and its API did not provide a
reliable, repeatable service for the project: requests could remain unanswered
or fail during consumption, and the sandbox requires payment whose pricing is
not transparent enough to budget a controlled campaign. These conditions do
not provide the technical guarantee required for the current demonstration.

The local OpenSSL authority issues real RFC 3161 tokens with an RSA-3072 TSA
certificate restricted to the `timeStamping` extended key usage. Its tokens
are accepted by `openssl ts -verify` and are exercised by deterministic
integration tests.

## Decision

The current delivery uses `LocalOpensslTsa` as its timestamp authority for
the demonstration, the end-to-end script, and the independent evidence
package. The Cincel adapter remains available as an optional future
integration point, but no paid sandbox campaign is required or claimed by
this delivery.

The application must never silently switch authorities. The issuer and trust
anchor remain explicit in every verification flow. A future PSC integration
requires a provider with a stable service contract, reproducible access, and
transparent pricing before it can be treated as project evidence.

## Consequences

- The technical RFC 3161 flow is reproducible without network access or
  external cost.
- The local TSA demonstrates protocol interoperability, but it does not
  provide independent third-party trust or a NOM-151 constancy from an
  authorized PSC.
- The external-provider success-rate criterion is not measured and is outside
  the current delivery scope.
- Replacing the local authority later remains an adapter substitution behind
  `TimestampService`; the domain and verifier do not need to change.
