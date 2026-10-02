# ADR 0048: Private versioned deployment over SSH

## Status

Accepted for the private Linux x86_64 installation. First tagged activation
and a complete release pipeline require separate acceptance evidence.

## Context

The application combines a Rust executable, a static Astro/Svelte frontend,
PostgreSQL, Redis, pinned qpdf/FFmpeg/ffprobe and persistent signing/TSA material.
Development Compose credentials are unsuitable for a persistent installation. The target
account can manage its own systemd services without administering other projects.
Startup validates exact database catalogs and historical inventory, so swapping
an old executable after migration is not generally a valid database rollback.

## Decision

Accept only canonical `vMAJOR.MINOR.PATCH` tags without leading zeros or suffixes.
Resolve annotated/lightweight tags to commits, compare event and checkout,
run the existing Rust and Web workflows as reusable workflows, then package a
locked release build. Identify artifacts by version and full commit, with an
archive checksum, file inventory and migration fingerprint. Recheck the remote
tag before transfer. Failed prerequisites prevent packaging or deployment.

Use the existing self-hosted runners: validation and transfer on VPS1 and package
build on VPS3. Reused Rust/Web workflows retain their distributed jobs. Build
the native release on the Ubuntu 22.04 target host to match its GLIBC ABI. Bundle
the verified FFmpeg/ffprobe 9.0.2 executables alongside pinned qpdf and select
those exact paths at runtime; all remain covered by the required file inventory.

Use a deployment-account-only Ed25519 credential through GitHub encrypted secrets
and pin the server Ed25519 key obtained through an already verified SSH connection. Only
versioned PKI scripts/configuration belong in the package, never private keys,
runtime configuration or databases.

Run dedicated PostgreSQL, Redis, API and nginx services under the deployment
account's systemd user manager. The target is VPS3 with the non-root `qadra`
account, root `/home/qadra/qadra`, and SSH port 22022. Preserve the original
installation and existing CI runners. Enable lingering for reboot/logout
persistence.
All listeners bind to loopback; access uses an SSH tunnel. The frontend proxy
blocks Owner bootstrap; initial enrollment is an explicit local API operation.
The local TSA retains the limits in `docs/adr/0009-local-timestamp-authority.md`.

Keep private state outside immutable releases. Serialize activation with a host
lock and tag workflows without canceling active deployment. Reject automatic
downgrades and reuse of a version for another commit. Verify the archive before
stopping ingress. Stop the API, back up the database/private state, initialize a
new installation once, switch the current symlink and check dependencies, API,
proxy, frontend and served identity. Record the previous link after success.

Capture Redis identity state together with SQL and private material while API
and web are stopped. Match Redis process and directory to the private service,
validate its RDB and publish a completion marker only after durable writes.
A Redis failure aborts activation; an older SQL-only backup is not a complete
identity backup. Restoring historical SQL requires invalidating sessions and
challenges while preserving current failure/replay controls. Restoring a lost
Redis instance also requires handling AOF precedence and preserving absolute
expiry times. A historical snapshot cannot recover later security state;
`docs/deployment-backups.md` defines the maintenance boundary and acceptance.

If initial trust publication committed before the schema marker was written,
reconcile it before generating PKI material. Resume only for revision 1 valid
at PostgreSQL's current time with exact local/persisted CA and CRL DER bytes;
do not publish trust twice. Missing or different CA/CRL material, expired trust
or a later revision requires explicit maintenance, not replacement of the
published authority or revocation list.


On failure, restore and check the previous release while still failing CI.
If recovery fails, stop ingress. Rollback never restores old data automatically.
Refuse automatic upgrades/rollbacks across migration fingerprints: these need
an explicit maintenance and restoration plan. Service/controller updates are
administrative installations, separate from application bundle activation.

## Consequences

GitHub Release publication is independent of application deployment. Invalid
`v*` tags can start failed validation runs but cannot deploy. Concurrency can
replace older pending runs; the server also rejects delayed version downgrades.

No privileged SSH account, public HTTP listener or deployment-time compiler is
needed. The deployment account can access its application secrets and databases;
the SSH key authorizes that account's full scope. Shared server services remain
separate, but CI and the application share VPS3 resources; service limits are
ceilings rather than reservations and need concurrent-load measurement. Health
proves readiness and version identity, not every business flow. Historical tests
on the original server do not establish VPS3 activation or rollback acceptance.

Migrations, certificate/CRL renewal, backup retention and external backup storage
require documented operation. Coverage, native admission and backend tests are
not bypassed to accelerate releases.
