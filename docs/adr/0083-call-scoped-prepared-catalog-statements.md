# 0083 - Call-scoped prepared statements for catalog validation

## Status

Accepted. The focused catalog client tests and representative schema, role and
permission rejection suites pass; the wall-clock saving per opening remains to
be measured against the current branch before it is claimed.

## Context

Every ordinary PostgreSQL opening, every consumer reconnection and the scoped
startup connection run `validate_runtime` in `crates/infrastructure/src/postgres.rs`
completely. It checks the catalog of every family, then the runtime role and
its privileges, then the immutable inventories. With empty data one opening
issues about 1,557 extended-protocol statements, roughly 1,450 of them in the
catalog and role checks.

The validation code calls `query`, `query_one` and `query_opt` with SQL text.
The PostgreSQL client prepares a new named statement for every such call: a
Parse and Describe round trip, then Bind and Execute, then Close. Only about 270
distinct texts exist per opening, so most of those Parse round trips repeat a
text already parsed moments before on the same connection. The checks run in a
fixed order and each depends on the previous one passing, so their round trips
are serialized.

`docs/adr/0062-scoped-postgres-startup-validation.md` requires that ordinary
openings and reconnections keep running the complete validation and forbids any
static, thread-local or process-wide validation cache. The SQL of the guards
must not change. `docs/adr/0045-single-pass-ci-and-dispatch-measurements.md`,
`docs/adr/0046-cached-ci-test-suites.md` and `docs/adr/0050-isolated-test-slots.md`
likewise keep production startup validation complete and uncached; they reduce
test cost only by reusing explicitly opened fixtures.

## Decision

Introduce `CatalogClient` and `PreparedCatalog` in
`crates/infrastructure/src/postgres/catalog_client.rs`. The catalog and runtime
role checks reached from `validate_runtime` accept any `CatalogClient`. A direct
`postgres::Client` or `postgres::Transaction` implements it by delegating to the
same client methods used before, so every other caller keeps its exact
behavior.

`validate_runtime` wraps its connection in a new `PreparedCatalog` for the
catalog checks, `SELECT current_user` and the runtime role checks. It borrows
the connection mutably for that block only. It prepares a statement the first
time it sees an exact SQL text and executes later calls with the same text
through that statement. A failed Parse is not remembered. The trait exposes
only `query`, `query_one` and `query_opt`, the methods these checks use; they
issue no `execute`, `batch_execute` or transaction calls.

Prepared statements are protocol state scoped to one `validate_runtime` call
on one connection. They hold query plans, never query results: every check is
executed again with its parameters, observes the current catalog, and fails
with the same error as before. The statement map is dropped when the block
ends; the client then closes each statement before the inventory checks start.
The inventory checks keep using the direct connection and its transactions.
There is no static, thread-local, shared or cross-connection cache, and each
opening, reconnection and startup validation still runs every query. SQL
text, parameters, call order, error messages and error mapping are unchanged.

## Consequences

One opening should send about 1,270 fewer Parse round trips: about 200 Parse
messages remain for the catalog and role checks plus about 90 for the inventory,
instead of about 1,557. Executions keep the same ordered sequence of statements
and parameters. The change is accepted when that sequence is identical, the
complete infrastructure suite passes and the wall-clock time of one opening
drops by at least 15 percent on the same host. A larger saving is only claimed
if the server log shows that repeated executions also reduce Bind cost, for
example because PostgreSQL adopts generic plans. Session settings such as
`plan_cache_mode` are not changed.

Tests in `crates/infrastructure/src/postgres/catalog_client/tests.rs` guarantee
that a `PreparedCatalog` returns the same results and errors as a direct
client, prepares each exact text once per instance, prepares again in a new
instance, observes catalog changes between executions of the same statement and
leaves no prepared statement on the connection after it is dropped. Existing
rejection tests keep checking the first failure and its message.

A concurrent administrator change to a relation used by a prepared check is
handled by PostgreSQL plan invalidation. As in
`docs/adr/0062-scoped-postgres-startup-validation.md`, arbitrary concurrent
administrator DDL remains outside the validation contract.
