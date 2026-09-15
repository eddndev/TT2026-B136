-- Case-local people have immutable history, independent of access assignments.
DO $$ BEGIN
    IF current_setting('server_encoding')<>'UTF8' THEN
        RAISE EXCEPTION 'canonical participant values require UTF8 database encoding';
    END IF;
END; $$;

CREATE OR REPLACE FUNCTION participant_text_valid(value TEXT, maximum INTEGER)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    character_code INTEGER;
    position INTEGER;
    whitespace INTEGER[] := ARRAY[32,160,5760,8192,8193,8194,8195,8196,8197,
        8198,8199,8200,8201,8202,8232,8233,8239,8287,12288];
BEGIN
    IF value IS NULL OR char_length(value) NOT BETWEEN 1 AND maximum THEN RETURN FALSE; END IF;
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

-- Bind references to the installation schema, including during empty-search-path restore.
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.participant_values_is_canonical(
    name TEXT, role TEXT, organization TEXT, legal_status TEXT, directory_status TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
BEGIN
    RETURN %1$I.participant_text_valid(name,200)
        AND %1$I.participant_text_valid(role,80)
        AND (organization IS NULL OR %1$I.participant_text_valid(organization,200))
        AND (legal_status IS NULL OR %1$I.participant_text_valid(legal_status,160))
        AND directory_status IS NOT NULL AND directory_status COLLATE "C" IN ('active','archived');
END;
$$;
$definition$, current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.participant_values_bytes(
    name TEXT, role TEXT, organization TEXT, legal_status TEXT, directory_status TEXT)
RETURNS BYTEA LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    result BYTEA := convert_to('PART1','UTF8');
    value TEXT;
    encoded BYTEA;
BEGIN
    IF NOT %1$I.participant_values_is_canonical(name,role,organization,legal_status,directory_status) THEN
        RAISE EXCEPTION 'participant values must be canonical' USING ERRCODE='23514';
    END IF;
    FOREACH value IN ARRAY ARRAY[name,role] LOOP
        encoded := convert_to(value,'UTF8');
        result := result || decode(lpad(to_hex(octet_length(encoded)),8,'0'),'hex') || encoded;
    END LOOP;
    FOREACH value IN ARRAY ARRAY[organization,legal_status] LOOP
        IF value IS NULL THEN result := result || decode('00','hex');
        ELSE
            encoded := convert_to(value,'UTF8');
            result := result || decode('01','hex')
                || decode(lpad(to_hex(octet_length(encoded)),8,'0'),'hex') || encoded;
        END IF;
    END LOOP;
    RETURN result || CASE directory_status WHEN 'active' THEN decode('00','hex') ELSE decode('01','hex') END;
END;
$$;
$definition$, current_schema());
END; $install$;

CREATE TABLE IF NOT EXISTS case_participants (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL,
    initial_revision BIGINT NOT NULL DEFAULT 1,
    CONSTRAINT participant_case_fk FOREIGN KEY(case_id) REFERENCES cases(id),
    CONSTRAINT participant_initial_revision CHECK(initial_revision=1)
);
CREATE TABLE IF NOT EXISTS case_participant_revisions (
    participant_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    display_name TEXT NOT NULL,
    procedural_role TEXT NOT NULL,
    organization TEXT,
    legal_status TEXT,
    directory_status TEXT NOT NULL,
    values_digest BYTEA NOT NULL,
    changed_at TEXT NOT NULL,
    changed_by UUID NOT NULL,
    changed_by_email TEXT NOT NULL,
    PRIMARY KEY(participant_id,revision),
    CONSTRAINT participant_root_fk FOREIGN KEY(participant_id) REFERENCES case_participants(id),
    CONSTRAINT participant_actor_fk FOREIGN KEY(changed_by) REFERENCES users(id),
    CONSTRAINT participant_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    CONSTRAINT participant_first_active CHECK(revision<>1 OR directory_status COLLATE "C"='active'),
    CONSTRAINT participant_canonical CHECK(participant_values_is_canonical(display_name,procedural_role,organization,legal_status,directory_status)),
    CONSTRAINT participant_digest CHECK(values_digest=pg_catalog.sha256(participant_values_bytes(display_name,procedural_role,organization,legal_status,directory_status)))
);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid='case_participants'::regclass
        AND conname='participant_first_revision_fk') THEN
        ALTER TABLE case_participants ADD CONSTRAINT participant_first_revision_fk
            FOREIGN KEY(id,initial_revision) REFERENCES case_participant_revisions(participant_id,revision)
            DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS participant_case_order ON case_participants(case_id,id);

CREATE OR REPLACE FUNCTION preserve_participant_history() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF OLD IS DISTINCT FROM NEW THEN
        RAISE EXCEPTION 'participant roots and revisions are immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_participant_sequence() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE latest_revision BIGINT;
BEGIN
    -- Match crates/infrastructure/src/audit_postgres.rs before reading the head.
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT MAX(revision) INTO latest_revision FROM %1$I.case_participant_revisions
        WHERE participant_id=NEW.participant_id;
    IF NEW.revision IS DISTINCT FROM COALESCE(latest_revision+1,1) THEN
        RAISE EXCEPTION 'participant revisions must be contiguous' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
$definition$, current_schema());
END; $install$;
DROP TRIGGER IF EXISTS participant_root_immutable ON case_participants;
CREATE TRIGGER participant_root_immutable BEFORE UPDATE ON case_participants
    FOR EACH ROW EXECUTE FUNCTION preserve_participant_history();
DROP TRIGGER IF EXISTS participant_revision_immutable ON case_participant_revisions;
CREATE TRIGGER participant_revision_immutable BEFORE UPDATE ON case_participant_revisions
    FOR EACH ROW EXECUTE FUNCTION preserve_participant_history();
DROP TRIGGER IF EXISTS participant_sequence ON case_participant_revisions;
CREATE TRIGGER participant_sequence BEFORE INSERT ON case_participant_revisions
    FOR EACH ROW EXECUTE FUNCTION enforce_participant_sequence();
REVOKE ALL ON case_participants,case_participant_revisions FROM PUBLIC;
REVOKE ALL ON FUNCTION participant_text_valid(TEXT,INTEGER),
    participant_values_is_canonical(TEXT,TEXT,TEXT,TEXT,TEXT),
    participant_values_bytes(TEXT,TEXT,TEXT,TEXT,TEXT),
    preserve_participant_history(),enforce_participant_sequence() FROM PUBLIC;
