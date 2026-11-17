# 0090 - Audit event lookup indexes

## Status

Accepted.

## Context

`audit_events` had two indexes: the primary key on `sequence` and
`audit_events_chronological` for the Owner audit query
([ADR 0061](0061-bounded-owner-audit-query.md) and
[migration 0027](../../migrations/0027_audit_query_projection.sql)). The opening
catalog check required exactly those two.

Several stores verify that a stored record is covered by its audit event before
they trust it. They look for an event by exact resource and action, by a
resource prefix, or by action alone:

- precautionary hearings: the marker count by exact resource and action, the
  orphan checks `ph1:%:hearing:<id>:operation:%`, `ph1:%:operation:<id>:%` and
  `ph1:case:<id>:hearing:%`
  (`crates/infrastructure/src/precautionary_hearing_postgres/audit.rs`);
- resource hearings: the creation marker by `action='resource_hearing.registered'`
  and exact resource, and by an operation substring
  (`crates/infrastructure/src/resource_hearing_postgres/replay.rs`);
- the measure decision and administration adapters, by action and a resource
  pattern.

These lookups run inside audited transactions, after
`pg_advisory_xact_lock` on the shared audit key
(`crates/infrastructure/src/audit_postgres.rs`). The activity alert scanner
reaches them for every precautionary and resource hearing it reconciles, and
the agenda reaches them for every such item on a page. Without an index each
one is a sequential scan of the whole history, so the time the shared lock is
held grows with the table. Agenda, deadline dispatch, the deadline worker and
the audit query wait at most one second for that lock; a longer hold fails them
with SQLSTATE 55P03, reported as `busy` and answered as `500 internal_error`.
On a copy of 4.47 million events each lookup took 0.3 to 0.8 s and a
precautionary hearing reconciliation ran about three of them.

The value distribution is very skewed: almost every event comes from a few
dozen technical resources and two actions, while the functional markers these
checks look for are rare.

## Decision

[Migration 0054](../../migrations/0054_audit_event_lookups.sql) adds two plain
btree indexes:

- `audit_events_action_resource (action, resource)`, for exact action and
  resource, action with a resource pattern, and action alone;
- `audit_events_resource (resource)`, for exact resources and anchored resource
  prefixes without an action predicate.

Both use the default operator class and the column collation. Every Qadra
database is created with `initdb --no-locale` and restore admission requires the
`C` collation, so PostgreSQL turns an anchored `LIKE 'ph1:...'` into an index
range without `text_pattern_ops`. A trailing `sequence` column is not added: it
would defeat btree deduplication, and the selected rows are few.

The opening catalog check now requires exactly four indexes on
`audit_events` and validates each new one like the existing ones: btree,
valid, not unique, not partial, without expressions or included columns,
ascending, default operator class and collation, and the exact column list.

The index is created by the ordinary migration. `database migrate` applies every
migration in one transaction, so `CREATE INDEX CONCURRENTLY` is not available;
the build holds a `SHARE` lock on `audit_events`, which blocks appends until
the migration commits. It runs in the maintenance window with ingress, API and
web stopped.

## Consequences

The coverage lookups become index or index-only scans whose cost depends on the
matching rows, not on the history size. On the same 4.47 million event copy
they took under 1 ms and the two indexes took about 6 s to build and 34 MB each.
Each audit append maintains two more index entries.

A binary built before this decision counts two indexes and refuses a migrated
database; a binary built after it refuses a database without the indexes.
Creating the indexes by hand under an older binary therefore stops every
consumer that revalidates the catalog when it reconnects. Going back to an
older binary is explicit maintenance: with writers stopped, drop both indexes
and reactivate the earlier release. The migration changes no rows, hashes or
privileges, so audit events written in between are kept.

The full scan of `audit_events` that recomputes the generated timestamp
projection at opening (`audit_query_schema::validate_inventory`) is not served
by these indexes and still grows with the table.
