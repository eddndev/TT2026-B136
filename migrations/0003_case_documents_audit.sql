CREATE TABLE IF NOT EXISTS audit_events (
    sequence BIGINT PRIMARY KEY CHECK (sequence >= 0),
    timestamp TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    chain BYTEA NOT NULL CHECK (octet_length(chain) = 32)
);

CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id),
    version BIGINT NOT NULL CHECK (version BETWEEN 1 AND 4294967295),
    name TEXT NOT NULL,
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    vault BYTEA NOT NULL,
    evidence JSONB CHECK (jsonb_typeof(evidence) = 'object')
);

CREATE INDEX IF NOT EXISTS documents_case_id_idx ON documents(case_id, id);

CREATE OR REPLACE FUNCTION preserve_document_evidence() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(OLD.id, OLD.case_id, OLD.version, OLD.name, OLD.digest, OLD.vault)
        IS DISTINCT FROM ROW(NEW.id, NEW.case_id, NEW.version, NEW.name, NEW.digest, NEW.vault)
        OR (OLD.evidence IS NOT NULL AND OLD.evidence IS DISTINCT FROM NEW.evidence) THEN
        RAISE EXCEPTION 'document identity and captured evidence are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS documents_preserve_evidence ON documents;
CREATE TRIGGER documents_preserve_evidence BEFORE UPDATE ON documents
    FOR EACH ROW EXECUTE FUNCTION preserve_document_evidence();

CREATE TABLE IF NOT EXISTS migration_receipts (
    fingerprint TEXT PRIMARY KEY,
    document_count BIGINT NOT NULL CHECK (document_count >= 0),
    audit_count BIGINT NOT NULL CHECK (audit_count >= 0),
    audit_head TEXT NOT NULL
);

REVOKE ALL ON audit_events, documents, migration_receipts FROM PUBLIC;
