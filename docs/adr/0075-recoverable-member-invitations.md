# Recoverable member invitations

## Status

Accepted. Native HTTP acceptance and focused recovery checks passed.
Integration and public email activation are recorded separately.

## Context

Direct member creation gives the administrator a password and one-time MFA
material for another account. It does not implement an invitation accepted by
its recipient. The existing editor can reconcile an interrupted administrator
session, but cannot recover an enrollment whose response was lost.

The deployment serves one office. There is no commercial plan or user quota.
Invitation registration uses the existing email, password and MFA fields.
No additional personal profile is required.

## Decision

An authenticated active Owner may invite a Litigator or Paralegal. Existing
accounts, including inactive accounts, are rejected; their access lifecycle
remains in member administration. An existing pending invitation is recovered
or explicitly resent instead of creating another account.

Each issuance has a random 256-bit capability, a 24-hour lifetime and a distinct
revision. The database stores its purpose-separated digest. The mail payload
is encrypted and bound to the invitation identity and revision. Owner views,
logs and audit never disclose the capability or MFA material. Explicit resend
invalidates the previous capability and incomplete enrollment atomically.

The recipient chooses a password and receives MFA enrollment material. That
material is encrypted durably so the same valid capability and password can
recover an interrupted response without replacing the pending credentials.
No usable user or normal session exists before TOTP confirmation. Completion
commits the account, capability consumption, audit and a secret-free receipt
in one transaction. Replaying a completed operation returns its receipt, never
another account or another secret. The recipient then uses normal login.

Owner commands use operation identifiers to reconcile uncertain responses.
Database mutations revalidate current authority and expiry after acquiring the
existing audited mutation lock. Concurrent acceptance, direct creation and
resend must preserve the unique account and reject obsolete observations.

Invitation mail uses the existing bounded transport with a durable delivery
intent and stable provider idempotency key. A lost provider response is an
uncertain result, not proof that the message was not sent. Retries stay within
the provider's 24-hour [idempotency window](https://resend.com/changelog/idempotency-keys);
later redelivery requires explicit resend.
Public mail activation remains an explicit operational decision. Acceptance
uses a controlled mail transport and isolated services.

Restoration must prevent old pending capabilities from becoming usable again
while preserving accepted account history and receipts. Existing credentials,
assignments, direct enrollment, recovery and the accepted production data are
preserved.

## Consequences

The new flow reuses password hashing, TOTP, recovery codes, authenticated
secret encryption, identity authorization and audited PostgreSQL transactions.
It adds invitation state and delivery recovery without replacing those systems.
Expired or superseded links cannot resume MFA; the Owner must resend. A failed
mail attempt must remain visible without creating a duplicate invitation.

This decision does not introduce billing, multiple offices per deployment,
extra personal fields, certificate issuance through the invitation UI, or new
legal workflows. Reproduced acceptance and operating instructions accompany the
completed implementation before integration.
