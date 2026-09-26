# 0048: Scope migration coordination to one PostgreSQL schema

## Status

Accepted.

## Context

Schema initialization applies many migrations in one transaction. Integration
fixtures create separate schemas in one database so their records and roles do
not overlap. The previous advisory lock used one key for the whole database,
causing independent fixture initialization to wait in a queue. Two adapters
initializing the same schema must still coordinate so neither observes a
partially applied migration.

## Decision

Use a two-integer transaction advisory key made from a fixed migration class
and the current schema's PostgreSQL namespace OID. Resolve the OID inside the
initialization transaction and fail if the schema does not exist. Keep all
migrations in that transaction. Test that two adapters targeting the same
schema wait on one lock and that a different schema can initialize while the
first schema's key is held. Audit mutations retain their separate database-wide
lock; this change does not alter audit ordering.

## Consequences

Independent schemas can migrate concurrently while PostgreSQL continues to
enforce its own catalog locks. One schema still has atomic, serialized
initialization. Deployments that overlap binaries using the old database-wide
key and this new key must stop old migration processes before starting new
ones; the two key formats do not coordinate. Measure CI before claiming a
runtime improvement, and keep the runner's resource limits unchanged.
