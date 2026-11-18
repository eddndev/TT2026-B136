-- Lookup indexes for the audit coverage checks that run under the shared audit
-- lock: docs/adr/0090-audit-event-lookup-indexes.md. Both use the default btree
-- operator class and column collation; the database collation is C, so a
-- prefix LIKE becomes an index range. Every migration runs in one transaction,
-- so the build takes a SHARE lock on audit_events and blocks appends until it
-- commits. Repeating this migration changes nothing.
CREATE INDEX IF NOT EXISTS audit_events_action_resource ON audit_events(action, resource);
CREATE INDEX IF NOT EXISTS audit_events_resource ON audit_events(resource);
