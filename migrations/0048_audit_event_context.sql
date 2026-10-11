-- Nullable metadata leaves historical canonical bytes and hashes unchanged.
ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS context_json TEXT
    CONSTRAINT audit_events_context_json_check
    CHECK (octet_length(context_json) <= 1024);
