# Verified exact document content and durable integrity incidents

## Status

Accepted design; implementation and acceptance are in progress. Results belong
in `docs/verification-report.md`, not in this decision record.

## Context

The online document workflow preserves encrypted immutable content versions,
metadata, sealing and evidence export. Its evidence ZIP requires a seal. Users
also need the original file before sealing, with the same case authorization
and no release of rejected content. A content download must not imply a valid
signature, certificate or timestamp.

Content validation may fail after an authorized encrypted snapshot was loaded.
Rolling back a successful-read transaction cannot also preserve a notification
of that failure. Activity alerts concern hearings and deadlines; their subjects,
eligibility, preferences and episodes do not describe document incidents.

## Decision

Add an independent content workflow over the existing document store, identity,
cryptographic processor and clock. It accepts an exact case/document/version,
loads with current authorization, validates AES-GCM with identity/version AAD
and SHA-256, reauthenticates the full principal and commits an exact audited
access before returning plaintext. Pending and sealed content use the same
content checks. Evidence verification remains a separate operation.

The final access transaction rechecks active role, case membership and immutable
content. A newer content version does not redirect this request. Concurrent
sealing may add evidence without changing the validated content. The event is
`document.content_authorized`: it does not assert delivery to a browser or disk.
Sessions in Redis and authorization in PostgreSQL do not form a distributed
transaction; logout after reauthentication cannot recall bytes already released.

Read at most 16 MiB of plaintext through this route. PostgreSQL checks encrypted
length before materializing the vault and repeats the bound in the selecting
statement. The cryptographic processor verifies the envelope bound independently.
A separate content capacity budget uses the configured maximum request count.
The permit follows the zeroizing plaintext buffer through blocking preparation
and into the HTTP bytes owner, until the last chunk or clone is dropped.
A slow recipient cannot release this budget merely by completing the handler.

Larger historical records remain preserved and return a size error, without an
integrity incident. Reading does not repeat upload-format admission.

A validation rejection follows a separate transaction that commits the incident
and `document.content_rejected` audit together. It records immutable identities,
failure class, times and digests, never plaintext, ciphertext, tokens or keys.
The technical actor is `system:document-integrity`; the original requester is
historical provenance, not an accusation or an impersonated mutation author.
A requester revoked after the authorized load does not erase that observation.
No successful content-access event is created for the rejected attempt.

An observation UUID identifies one detection attempt. Exact replay returns the
original receipt; changed reuse conflicts. Distinct attempts are separate
incidents, including repeated detection of the same stored defect. An incident
is not deleted or rewritten when the underlying file is later restored.

The append-only incident table is the durable internal notice. Active Owners
can list and inspect it, with current authorization and audit before disclosure.
Qadra checks for its existence at session entry and offers an explicit refresh
and an Owner-only inbox. There is no acknowledgement, resolution, email or push
delivery claim in this contract. Incident queries do not decrypt a rejected file.

Authentication failure can result from a wrong KEK or modified wrapped material.
The category states an observation, not its cause. Technical/database failures
remain distinct. If the incident commit fails, content still cannot be returned;
the response must not claim that a durable notice was recorded.

## Consequences

Users can retrieve a historical original without signing it first. No response
body containing file bytes is built until authorization and audit succeed.
Buffers use zeroization through application processing; the HTTP handoff does
not promise erasure from network stacks or a recipient's computer.

Internal notices survive disconnected Owner sessions and database restoration.
They require strict schema/grants/inventory and backup coverage. A database
outage cannot guarantee a durable incident in that same database; an external
durable spool would require a separate design.

This change retains Client denial. Granting Client access requires a separate
policy and tests per resource. It does not establish legal authenticity, confirm
an attack, detect a coherent rollback, or resolve external audit anchoring in
`docs/adr/0007-audit-chain-anchoring.md`.
