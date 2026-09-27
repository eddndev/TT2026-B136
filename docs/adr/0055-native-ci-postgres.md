# 0055: Disposable native PostgreSQL for dedicated CI

## Status

Accepted for a measured rollout; complete concurrent CI validation is pending.

## Context

The complete dedicated campaign passed in 13m12s, including 738.57 seconds of
Rust tests. Warm compilation took 0.19 seconds and coverage reporting about six
seconds. A costly catalog-integrity test performs 53718 SQL calls. With the same
instrumented test executable and fresh PostgreSQL 16 databases, it passed in
40.747 seconds through the rootless Alpine container and 21.103 seconds through
the native Ubuntu PostgreSQL package. Both used the same disposable durability
settings and recorded no JIT work. SQL execution itself also fell from about
10.01 to 4.93 seconds. This comparison changes database distribution, storage
and transport together; it does not isolate network overhead as the sole cause.
The focused result does not establish a complete CI improvement.

## Decision

Run PostgreSQL 16 natively for the dedicated test job only. Keep a private,
fresh cluster for each invocation, random SCRAM credentials, a dynamically
selected loopback TCP port and no Unix socket. The native service runs as the
runner account in a transient user systemd unit with MemoryMax=6GiB, within
the existing shared account CPU and memory budget. Require the applied memory
limit and PostgreSQL major version before running tests. Keep the disposable
settings from `docs/adr/0047-disposable-postgres-ci.md` and the independent
identity, case and document databases per Nextest slot from
`docs/adr/0050-isolated-test-slots.md`.

`scripts/ci_native_postgres.py` owns the service lifetime and wraps the existing
coverage command. Preserve its exit status. On termination, stop test children
before the database; on success or failure, stop the service and remove only
its private cluster. Keep the PostgreSQL log outside that temporary cluster.
An unconditional workflow cleanup step handles a surviving ownership marker.
The service also has a maximum lifetime matching the existing 210-minute job
budget in case the supervising process is killed without running its cleanup.
Never redirect these settings or cleanup operations to a developer or deployed
database. No persistent native PostgreSQL service is required.

Keep Redis container isolation and its 128MiB limit. The non-dedicated fallback
and manual extended campaign retain their PostgreSQL containers. Compilation,
all tests, assertions, authentication, transaction behavior, timeouts and
coverage gates remain unchanged.

## Consequences

Dedicated runner provisioning now requires the native PostgreSQL 16 server
and client tools on PATH, plus a functioning user systemd manager. The account
slice must include that manager and all native services in its shared budget.
A runner migration requires those documented dependencies; no host address or
runner name is embedded in the helper. The temporary cluster contains no
persistent project data and every campaign starts with new databases.

Confirm the full test inventory, coverage, concurrent browser behavior and
active-window resource measurements before accepting the runtime improvement.
Record the native service's child memory events as well as the shared parent
budget. Do not infer full-suite timing by scaling the single-test result.
