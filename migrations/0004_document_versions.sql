-- Preserve the existing encrypted rows while introducing immutable series roots.
DO $$
DECLARE
    primary_columns TEXT[];
    primary_name TEXT;
BEGIN
    IF to_regclass('document_series') IS NULL THEN
        CREATE TABLE document_series (
            id UUID PRIMARY KEY,
            case_id UUID NOT NULL REFERENCES cases(id),
            first_available_version BIGINT NOT NULL
                CHECK (first_available_version BETWEEN 1 AND 4294967295),
            UNIQUE(id, case_id)
        );
        INSERT INTO document_series(id, case_id, first_available_version)
            SELECT id, case_id, version FROM documents;
    END IF;

    SELECT c.conname, array_agg(a.attname::text ORDER BY k.ordinality)
        INTO primary_name, primary_columns
        FROM pg_constraint c
        CROSS JOIN LATERAL unnest(c.conkey) WITH ORDINALITY k(attnum, ordinality)
        JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum
        WHERE c.conrelid = 'documents'::regclass AND c.contype = 'p'
        GROUP BY c.conname;
    IF primary_columns = ARRAY['id'] THEN
        EXECUTE format('ALTER TABLE documents DROP CONSTRAINT %I', primary_name);
        ALTER TABLE documents ADD PRIMARY KEY(id, version);
    ELSIF primary_columns IS DISTINCT FROM ARRAY['id', 'version'] THEN
        RAISE EXCEPTION 'unexpected document primary key';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE
        conrelid = 'documents'::regclass AND conname = 'documents_series_case_fk') THEN
        ALTER TABLE documents ADD CONSTRAINT documents_series_case_fk
            FOREIGN KEY(id, case_id) REFERENCES document_series(id, case_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE
        conrelid = 'document_series'::regclass AND conname = 'document_series_first_version_fk') THEN
        ALTER TABLE document_series ADD CONSTRAINT document_series_first_version_fk
            FOREIGN KEY(id, first_available_version) REFERENCES documents(id, version)
            DEFERRABLE INITIALLY DEFERRED;
    END IF;
END;
$$;

CREATE INDEX IF NOT EXISTS document_series_case_id_idx ON document_series(case_id, id);

CREATE OR REPLACE FUNCTION preserve_document_series() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF OLD IS DISTINCT FROM NEW THEN
        RAISE EXCEPTION 'document series identity and first version are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION enforce_document_version_sequence() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    first_version BIGINT;
    latest_version BIGINT;
BEGIN
    -- Match crates/infrastructure/src/audit_postgres.rs before reading the head.
    -- An advisory lock requires no UPDATE grant on the immutable series root.
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT first_available_version INTO first_version FROM document_series
        WHERE id = NEW.id AND case_id = NEW.case_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'document series and case do not exist'
            USING ERRCODE = '23503';
    END IF;
    SELECT MAX(version) INTO latest_version FROM documents WHERE id = NEW.id;
    IF NEW.version IS DISTINCT FROM COALESCE(latest_version + 1, first_version) THEN
        RAISE EXCEPTION 'document versions must append the next contiguous version'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS document_series_preserve_identity ON document_series;
CREATE TRIGGER document_series_preserve_identity BEFORE UPDATE ON document_series
    FOR EACH ROW EXECUTE FUNCTION preserve_document_series();

DROP TRIGGER IF EXISTS documents_version_sequence ON documents;
CREATE TRIGGER documents_version_sequence BEFORE INSERT ON documents
    FOR EACH ROW EXECUTE FUNCTION enforce_document_version_sequence();

REVOKE ALL ON document_series FROM PUBLIC;
REVOKE ALL ON FUNCTION preserve_document_series(), enforce_document_version_sequence() FROM PUBLIC;
