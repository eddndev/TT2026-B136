# ADR 0048: Private versioned deployment over SSH

## Status

Accepted for the private Linux x86_64 installation. First tagged activation
and a complete release pipeline require separate acceptance evidence.

## Context

The application combines a Rust executable, a static Astro/Svelte frontend,
PostgreSQL, Redis, pinned qpdf and persistent signing/TSA material. Development
Compose credentials are unsuitable for a persistent installation. The target
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

Use the provided Ed25519 credential through GitHub encrypted secrets and pin the
server Ed25519 key obtained through an already verified SSH connection. Only
versioned PKI scripts/configuration belong in the package, never private keys,
runtime configuration or databases.

Run dedicated PostgreSQL, Redis, API and nginx services under the deployment
account's systemd user manager. Enable lingering for reboot/logout persistence.
All listeners bind to loopback; access uses an SSH tunnel. The frontend proxy
blocks Owner bootstrap; initial enrollment is an explicit local API operation.
The local TSA retains the limits in `docs/adr/0009-local-timestamp-authority.md`.

Keep private state outside immutable releases. Serialize activation with a host
lock and tag workflows without canceling active deployment. Reject automatic
downgrades and reuse of a version for another commit. Verify the archive before
stopping ingress. Stop the API, back up the database/private state, initialize a
new installation once, switch the current symlink and check dependencies, API,
proxy, frontend and served identity. Record the previous link after success.

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
separate. Health proves readiness and version identity, not every business flow.

Migrations, certificate/CRL renewal, backup retention and external backup storage
require documented operation. Coverage, native admission and backend tests are
not bypassed to accelerate releases.
