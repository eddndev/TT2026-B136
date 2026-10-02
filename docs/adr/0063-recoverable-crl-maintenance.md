# ADR 0063: Recoverable CRL maintenance for a private deployment

## Status

Accepted for implementation after isolated controller and native SQL/OpenSSL
acceptance. Installation and operational acceptance of the updated controllers
remain required before operating the deployed authority.

## Context

The API loads the operational CRL when it starts. Participant credential checks
also use an immutable PostgreSQL history of the authority and its CRL revisions.
The OpenSSL CA scripts generate seven-day lists and advance a filesystem counter.
SQL publication, the files, and systemd startup cannot share one transaction.
An interruption can therefore leave different material in memory, on disk, and
in the published trust head. Ordinary release rollback must not undo a committed
trust revision or reintroduce a retired revocation list.

The original deployment controller in
`docs/adr/0048-private-versioned-deployment.md` deliberately limits initialization
recovery to the first trust revision. Later renewal needs its own procedure.

## Decision

Keep renewal an explicit administrative command under the existing deployment
lock. Require the expected SQL revision and compare the persisted root and CRL
DER with the current filesystem before preparing any change. Preserve deployment
identity, the authority key, signer and TSA material, and all revoked serials.
Generate the candidate in a private operation directory using the existing CA
key, without copying that key or changing the live CA index or counter.

Record the operation, public material identities, release identity, backup and
progress durably. Stop API and web and require the exact original SQL head, CA,
CRL and counter before capturing the complete SQL/Redis/PKI backup. Capture it
before installing the maintenance marker, so its configuration archive does not
restore a reference to an operation journal outside that archive. A retry without
a recorded backup may remove its own marker only after proving this same original
state with services stopped; an advanced counter, CRL or SQL head prohibits that
capture. No live trust changes occur in this interval.

Persist the backup fingerprints, install and sync the maintenance marker, stop
services again and reconcile the state before changing live trust.
Runtime startup, web readiness and release activation/rollback reject that
marker, including after reboot. They do not infer safety from its contents.

Advance the CRL counter monotonically and install the candidate atomically.
Publish it through the exact accepted executable using its existing expected
revision and audited PostgreSQL transaction. Re-read SQL to establish the exact
successor with the same deployment/root and candidate CRL; command exit status
or a journal claim alone is insufficient. Only after files and SQL agree may
the marker be removed and the same release restarted and checked.

On interruption, resume the recorded candidate rather than silently generating
another. If the expected old head remains, publication may be retried. If the
exact successor is already committed, do not publish it again. An unrelated
head, modified material, an expired candidate or unavailable SQL keeps ingress
closed for explicit recovery. Failed startup restores the maintenance marker
and stops ingress; it never restores an old SQL revision or decreases a counter.
Release symlinks and database history remain unchanged by renewal.

## Consequences

The maintenance record supports recovery across the filesystem/SQL boundary,
but it is not a distributed transaction. Availability is deliberately reduced
while concordance is unknown. A crash after confirmed concordance but before a
health result may require rechecking readiness without another publication.
Before the maintenance marker is installed, an interruption can expose only the
unchanged original trust pair. Complete backup restoration still requires its
own validity, state and readiness checks; an absent marker is not that acceptance.

Root or the deployment account can still alter private files outside the
controller. OpenSSL certificate issuance and revocation must use the same
maintenance lock; they must not race this operation. Expired roots, lost keys,
changed authority identity and unrelated trust history require a separate
restoration or authority migration, not this renewal command.

No timer, public network exposure, key rotation, test relaxation or automatic
SQL rollback is introduced by this decision. Scheduling and retention require
separate documented operational acceptance.
