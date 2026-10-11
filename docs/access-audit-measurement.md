# Native durable access measurement

## Protocol declared before execution

This protocol measures `AccessAuditService` with `PostgresAuditLog` and a real,
isolated PostgreSQL database. It does not execute the HTTP handlers or prove
authorization for the route names used as payloads. HTTP boundary coverage and
existing role/resource fixtures retain their separate acceptance. No result is
claimed by writing this protocol or its tests.

The first TSV block in [the access matrix](access-audit-matrix.md) supplies exactly
222 unique functional method/path pairs. Every pair is submitted with each of
`access.granted`, `access.denied`, `access.rejected` and `access.failed`, in three
complete repetitions, for each concurrency of 1 and 8. This is
`222 * 4 * 3 * 2 = 5328` measured appends. Explicit HEAD rejections, health, assets
and unknown routes are outside this direct-writer population.

Each concurrency gets its own disposable schema and restricted runtime role.
Preparation commits 1000 chained internal events and retains their complete
contents. A new writer connection and one `AccessAuditService` are then shared
by the measured workers. Connection validation and history preparation are
outside the timer. PostgreSQL must report `fsync=on`, `full_page_writes=on` and
`synchronous_commit=on`; otherwise the measurement does not start.

The worker queue is bounded by the declared concurrency. Each batch contains at
most that many jobs, and the next batch is submitted only after every current
job has returned. A monotonic timestamp is captured immediately before each
enqueue and after `record` returns. The elapsed interval includes bounded-worker
dispatch, the service writer mutex, the database audit lock, INSERT and durable
COMMIT. Worker-start time separately exposes the dispatch wait. This test has one
database connection, no connection pool, and does not measure the HTTP admission
layer or Tokio blocking pool.

The first batch on each fresh writer is labelled `fresh_writer_first_batch`:
one sample for concurrency 1 and eight for concurrency 8. The remaining samples
are labelled `warm_writer`. PostgreSQL buffers and operating-system caches have
already been used during preparation; neither label claims a cold disk or cold
database cache. The denied payloads have unavailable user identity; other outcomes
carry the fixture Owner UUID. Every payload has a separate request UUID, an HTTP
source and the synthetic address `192.0.2.80`. These are declared writer inputs,
not observations of a network request or a permission decision.

Every sample retains its route, outcome, repetition, concurrency, context,
monotonic enqueue/start/return offsets, elapsed duration, durable-return result,
failure category and the committed event sequence/hash when present. After each
population, a new connection reloads the chain, checks the unchanged historical
prefix, verifies every hash and reconciles successful request identities one to
one with stored events. This is connection reopening, not a PostgreSQL process
restart or backup/restore campaign; those retain separate acceptance.

The receipt includes individual samples, counts, minimum, maximum and p50/p95/p99
using nearest rank: sorted value at `ceil(p * n) - 1`. Failed appends remain visible
and are excluded from successful-duration quantiles. **Every successful sample
must be strictly below 500 ms**; a failure, missing event, altered prefix, broken
chain or any sample at/above 500 ms fails the campaign. The receipt is written
before the final acceptance assertion, so threshold failures retain evidence.
No average or percentile substitutes for this per-sample criterion.

## Inputs and execution

The coordinator runs this alone and supplies:

- `CASE_TEST_DATABASE_URL`: an isolated disposable PostgreSQL database.
- `AUDIT_ACCESS_REVISION`: the exact measured revision, including a declaration
  when local modifications are present.
- `AUDIT_ACCESS_ENVIRONMENT`: a JSON object containing `os`, `cpu`, `memory_bytes`,
  `postgres_disk`, `build_profile` and `temporary_filesystem`. Values describe the
  actual host/storage and build; the test does not infer them from timing.
- `AUDIT_ACCESS_RECEIPT`: a new absolute JSON path in an existing private output
  directory. Existing receipts are never overwritten.
- `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1` and the private disk-backed `TMPDIR`
  required by the repository's verification guidance.

The PostgreSQL version, durability settings, WAL sync method, initial audit
relation size and history count are captured in the receipt for each population.
The test sources are [native acceptance](../crates/infrastructure/tests/audit_access_postgres.rs)
and [bounded measurement support](../crates/infrastructure/tests/audit_access_measurement_support.rs).

```bash
cargo test -p infrastructure --test audit_suite_1 \
  audit_access_postgres::durable_access_catalog_latency_with_bounded_workers \
  -- --ignored --exact --nocapture
```

Two separate native tests exercise real persistence of all four outcomes and
the existing Owner-only history boundary: a Litigator is denied, and an Owner
whose account was revoked is denied without another read event. They do not
stand in for the 222 routes' business authorization fixtures.

## Integrated HTTP campaign declared before execution

The optional receipt from `scripts/api-access-audit-demo.py` supplements the
direct-writer population with `scripts/api-demo.sh` against real composed servers.
The declared population covers the initial server, its concurrent seal competitor,
and the imported server before and after its TERM/INT restarts: five server
generations, at most two concurrent servers during the seal race. The imported
history preserves initial access events and adds the expanded workflow fixtures.
Collection occurs after all five generations have stopped and before the full
backup/restore, from exactly their five logs and the imported database. It never
mixes the subsequent restored-server population into this receipt.

The maximum is eight admitted requests and two blocking handler operations per
server, not a global shared limit across both processes. Certificate login is
disabled initially and enabled in the imported generations. Individual signing
is composed. Native PostgreSQL, Redis, local PKI and existing role/resource
fixtures are used; email delivery is disabled. Workflow history grows as the
fixtures run; this is not the writer experiment with a fixed 1000-event prefix.
The receipt captures total history and the prefix before its first access event.
Samples do not establish sustained concurrency eight or contention at every
worker acquisition. The two-server race exercises distinct writers against the
same database lock; the restart checks are also functional evidence.

The first collector wiring stopped at the initial server and omitted its seal
competitor; that population was corrected before the expanded campaign. The
first two attempts retained only generic collection failures, with no accepted
measurement receipt. A later native diagnostic established that passing a full
URI as `PGDATABASE` made psql treat it as a literal database name. Explicit libpq
environment fields corrected that defect, with a separate disposable PostgreSQL
connection check before repeating this unchanged five-generation population.
The failed measurements are not counted as performance acceptance.

When `TT_ACCESS_AUDIT_RECEIPT` is configured, all five measured generations use
`RUST_LOG=warn,qadra::audit_commit=info`. The collector accepts only the exact
`qadra::audit_commit` target, with its server request UUID, canonical operation,
durable result and `elapsed_micros`. The production timer begins before dispatch
to the Tokio blocking pool and ends after the access writer returns from its
commit. It includes blocking-pool queueing, the access writer mutex, database
work and commit. It excludes the preceding handler and HTTP admission time.
The single access writer connection has no PostgreSQL pool checkout; the
configured two-operation work budget belongs to the handler, outside this
timer. The measurement must not describe that excluded wait as included.

The collector uses a read-only PostgreSQL snapshot with the administrative
connection supplied in `TT_ACCESS_AUDIT_DATABASE_URL`. It selects only the four
`access.*` actions, sequence, canonical method/route and typed context. Every
commit-log UUID must have exactly one corresponding stored access event, and
every stored access event in this declared population must have exactly one log.
The operation must agree and belong to the versioned functional route catalog.
Context must retain `source=http`, a valid observed IP and the same request UUID;
an established user UUID remains exact and unavailable identity remains null.
MFA's shared request identity can therefore be reconciled without storing
challenges, codes, credentials or diagnostic-log payloads.

The collector never writes a connection URL, SQL diagnostic, free-form actor,
full resource string, request body, query, bearer or raw log line to its JSON.
It stores the explicit sample fields, public typed context, revision, declared
environment and PostgreSQL durability settings. The administrative PostgreSQL URI is decoded into explicit libpq connection
environment fields, outside command arguments; no URI query or fragment is accepted.
In particular, a full URI is never used as a literal `PGDATABASE` name. The receipt is
a new absolute path with mode 0600; existing evidence is never overwritten.
`AUDIT_ACCESS_REVISION` and `AUDIT_ACCESS_ENVIRONMENT` have the same meaning and
required environment fields as the direct-writer experiment above.

Every sample must report durable success and **strictly less than 500000 us**.
A sample at the boundary fails. Threshold failures retain their individual
samples in a rejected receipt before the collector exits unsuccessfully.
Malformed target records or inconsistent database reconciliation fail closed.
The collector retains a separate private `.inputs.json` containing only the
validated public fields before reconciliation, so it can be diagnosed and
replayed without repeating the native workflows. A fixed diagnostic identifies
validation failures without exposing connection strings or raw source records.
An optional measurement failure does not skip the remaining functional restore
checks; the complete command still exits unsuccessfully after those checks.
PostgreSQL must report all three durability settings enabled. The receipt lists
distinct observed operations and every missing catalog operation; it never
claims universal route coverage. Complete 222-pair boundary coverage is supplied
by the HTTP unit suite, while business permissions remain covered by their
existing workflow fixtures. Neither that unit coverage nor direct-writer timing
is substituted for these observed native HTTP commits.
