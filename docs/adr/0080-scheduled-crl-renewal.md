# ADR 0080: Scheduled CRL renewal near expiry

## Status

Accepted and implemented on 2026-10-21 at the author's explicit request.
Installation of the units on the deployment host, its alert configuration and
operational acceptance are recorded separately in
[CRL maintenance](../deployment-crl.md). It supersedes the
statements in [ADR 0063](0063-recoverable-crl-maintenance.md) and
[ADR 0078](0078-separate-ca-operator-and-recoverable-operations.md) that exclude
an automatic renewal schedule and backup retention. The recoverable maintenance
procedure of ADR 0063 is otherwise unchanged.

## Context

The internal authority issues seven-day CRLs. When the published list expires,
Owner certificate login, individual document signing and participant credential
checks stop admitting new operations until a person renews it. Renewal was a
manual command, so availability depended on someone remembering a weekly task.

Renewal stops the office API and web, captures a complete backup that includes
the authority key, installs the next list and publishes it as the next SQL trust
revision. The publication is monotonic and cannot be rolled back. An interrupted
operation leaves a fence that keeps ingress closed until a person resumes it.
Every backup is kept, so unattended renewals would grow without bound.

## Decision

A daily user timer on the deployment host runs `renew_crl.py auto` through the
approved controller launcher and generation. Under the deployment lock it:

1. Records `blocked` without stopping services when CRL maintenance, a restore
   fence or a CA operation is pending. It never resumes another operation.
2. Records `skipped` when the persisted trust remains valid for longer than the
   threshold, 72 hours by default.
3. Records `ca_expiring` without stopping services when the authority certificate
   expires before the published list, because another list would not extend
   validity. Replacing the authority remains a separate procedure.
4. Otherwise renews with the revision it has just observed as the expected
   revision, through the same prepare and continue steps as the manual command.
5. Records `busy`, a retryable outcome, when another process holds the lock and
   the published trust remains valid beyond the threshold. Inside the threshold,
   or when validity cannot be read, it records `busy_near_expiry` as a failure.

Each run writes a private receipt in `maintenance/crl-schedule` with outcomes,
revisions, validity and the backup path, without credential material. After a
successful scheduled renewal, only backups created by accepted scheduled
renewals are removed, keeping the newest four including the current one. A
backup is removed only when its receipt and operation journal identify the same
local backup directory. Manual, activation
and restoration backups are never removed by this policy.

A failure, block, interruption or invalid configuration marks the unit failed
and triggers a separate alert unit. The alert sends one generic message through
the configured Resend sender to the recipient in `config/crl-schedule.json`,
with no secrets. Its idempotency key is the systemd invocation of the failed
run when systemd exports it to OnFailure units (version 251 or later) and a
random key otherwise, and it states when that run left no receipt. A failed renewal remains
fenced with ingress closed, exactly like a failed manual renewal, until a person
inspects it and runs `resume`.

The renewal unit stops with SIGINT and mixed kill mode after 2400 seconds, so the
Python process reinstalls the fence before child commands are killed. A failure
to read validity afterwards or to apply retention is recorded with the accepted
renewal instead of replacing its outcome.

`renew_crl.py schedule-units` renders the timer and both services from the
launcher invocation that admitted it, including its interpreter, launcher, root
and inventory digest. The units therefore pin one controller generation.

## Consequences

The office API and web stop briefly about every four days, at a randomized time
after 09:00 UTC. Owner sessions that originated from a certificate must
reauthenticate after each revision. Renewal no longer depends on a person, but
recovery from a failure still does.

Installing a new controller generation must re-run `schedule-units` through the
new pin. Until then, both units fail with the previous pin and no alert can be
sent; the 72-hour threshold leaves at least two daily attempts before expiry.
The alert depends on outbound email and on the configured recipient.

Renewal generates the list from the current CA index. A completed local
revocation that is still pending publication is therefore published by the next
scheduled renewal; publishing from the portal remains the immediate path.

Renewal without stopping services would require reloading the CRL in the API and
a backup that does not require stopped ingress. That redesign is outside this
decision.
