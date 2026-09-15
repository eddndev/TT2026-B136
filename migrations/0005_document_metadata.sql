-- Organizational metadata has its own immutable history, independent of evidence.
DO $$ BEGIN
    IF current_setting('server_encoding')<>'UTF8' THEN
        RAISE EXCEPTION 'canonical document metadata requires UTF8 database encoding';
    END IF;
END; $$;

CREATE OR REPLACE FUNCTION document_metadata_text_valid(value TEXT, maximum INTEGER)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    character_code INTEGER;
    position INTEGER;
    whitespace INTEGER[] := ARRAY[32,160,5760,8192,8193,8194,8195,8196,8197,
        8198,8199,8200,8201,8202,8232,8233,8239,8287,12288];
BEGIN
    IF value IS NULL OR char_length(value) NOT BETWEEN 1 AND maximum THEN
        RETURN FALSE;
    END IF;
    IF ascii(left(value,1))=ANY(whitespace) OR ascii(right(value,1))=ANY(whitespace) THEN
        RETURN FALSE;
    END IF;
    FOR position IN 1..char_length(value) LOOP
        character_code := ascii(substr(value,position,1));
        IF character_code BETWEEN 0 AND 31 OR character_code BETWEEN 127 AND 159 THEN
            RETURN FALSE;
        END IF;
    END LOOP;
    RETURN TRUE;
END;
$$;

-- Bind project references to the installation schema for empty-search-path restores.
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.document_metadata_is_canonical(kind TEXT, classification TEXT, tags TEXT[])
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    tag TEXT;
    previous TEXT;
BEGIN
    IF (kind IS NOT NULL AND NOT %1$I.document_metadata_text_valid(kind,80))
        OR (classification IS NOT NULL AND NOT %1$I.document_metadata_text_valid(classification,80))
        OR tags IS NULL OR cardinality(tags)>20 THEN
        RETURN FALSE;
    END IF;
    IF cardinality(tags)=0 THEN RETURN TRUE; END IF;
    IF array_ndims(tags)<>1 OR array_lower(tags,1)<>1 THEN RETURN FALSE; END IF;
    FOREACH tag IN ARRAY tags LOOP
        IF NOT %1$I.document_metadata_text_valid(tag,40)
            OR (previous IS NOT NULL AND convert_to(previous,'UTF8')>=convert_to(tag,'UTF8')) THEN
            RETURN FALSE;
        END IF;
        previous := tag;
    END LOOP;
    RETURN TRUE;
END;
$$;
$definition$, current_schema());
END; $install$;

DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.document_metadata_bytes(kind TEXT, classification TEXT, tags TEXT[])
RETURNS BYTEA LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    result BYTEA := convert_to('DMETA1','UTF8');
    value TEXT;
    encoded BYTEA;
BEGIN
    IF NOT %1$I.document_metadata_is_canonical(kind,classification,tags) THEN
        RAISE EXCEPTION 'document metadata must be canonical' USING ERRCODE='23514';
    END IF;
    FOREACH value IN ARRAY ARRAY[kind,classification] LOOP
        IF value IS NULL THEN result := result || decode('00','hex');
        ELSE
            encoded := convert_to(value,'UTF8');
            result := result || decode('01','hex')
                || decode(lpad(to_hex(octet_length(encoded)),8,'0'),'hex') || encoded;
        END IF;
    END LOOP;
    result := result || decode(lpad(to_hex(cardinality(tags)),8,'0'),'hex');
    FOREACH value IN ARRAY tags LOOP
        encoded := convert_to(value,'UTF8');
        result := result || decode(lpad(to_hex(octet_length(encoded)),8,'0'),'hex') || encoded;
    END LOOP;
    RETURN result;
END;
$$;
$definition$, current_schema());
END; $install$;

CREATE TABLE IF NOT EXISTS document_metadata_revisions (
    document_id UUID NOT NULL,
    metadata_revision BIGINT NOT NULL,
    document_type TEXT,
    classification TEXT,
    tags TEXT[] NOT NULL,
    metadata_digest BYTEA NOT NULL,
    changed_at TEXT NOT NULL,
    changed_by UUID NOT NULL,
    changed_by_email TEXT NOT NULL,
    PRIMARY KEY(document_id,metadata_revision),
    CONSTRAINT document_metadata_series_fk FOREIGN KEY(document_id) REFERENCES document_series(id),
    CONSTRAINT document_metadata_actor_fk FOREIGN KEY(changed_by) REFERENCES users(id),
    CONSTRAINT document_metadata_revision_range CHECK(metadata_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT document_metadata_canonical CHECK(document_metadata_is_canonical(document_type,classification,tags)),
    CONSTRAINT document_metadata_digest CHECK(metadata_digest=pg_catalog.sha256(document_metadata_bytes(document_type,classification,tags)))
);

CREATE OR REPLACE FUNCTION preserve_document_metadata() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF OLD IS DISTINCT FROM NEW THEN
        RAISE EXCEPTION 'document metadata revisions are immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;

DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_document_metadata_sequence() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE latest_revision BIGINT;
BEGIN
    -- Match crates/infrastructure/src/audit_postgres.rs before reading the head.
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT MAX(metadata_revision) INTO latest_revision FROM %1$I.document_metadata_revisions
        WHERE document_id=NEW.document_id;
    IF NEW.metadata_revision IS DISTINCT FROM COALESCE(latest_revision+1,1) THEN
        RAISE EXCEPTION 'document metadata must append the next contiguous revision'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
$definition$, current_schema());
END; $install$;

DROP TRIGGER IF EXISTS document_metadata_preserve_history ON document_metadata_revisions;
CREATE TRIGGER document_metadata_preserve_history BEFORE UPDATE ON document_metadata_revisions
    FOR EACH ROW EXECUTE FUNCTION preserve_document_metadata();
DROP TRIGGER IF EXISTS document_metadata_sequence ON document_metadata_revisions;
CREATE TRIGGER document_metadata_sequence BEFORE INSERT ON document_metadata_revisions
    FOR EACH ROW EXECUTE FUNCTION enforce_document_metadata_sequence();

REVOKE ALL ON document_metadata_revisions FROM PUBLIC;
REVOKE ALL ON FUNCTION document_metadata_text_valid(TEXT,INTEGER),
    document_metadata_is_canonical(TEXT,TEXT,TEXT[]), document_metadata_bytes(TEXT,TEXT,TEXT[]),
    preserve_document_metadata(), enforce_document_metadata_sequence() FROM PUBLIC;
