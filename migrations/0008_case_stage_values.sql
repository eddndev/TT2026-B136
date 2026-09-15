-- Pure checks use the table's fixed composite type, with no free-form JSON storage.
CREATE OR REPLACE FUNCTION case_stage_support_valid(
    id UUID, version BIGINT, digest BYTEA, name TEXT, format TEXT, policy TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
BEGIN
    RETURN id IS NOT NULL AND version IS NOT NULL AND version BETWEEN 1 AND 4294967295
        AND digest IS NOT NULL AND pg_catalog.octet_length(digest)=32
        AND name IS NOT NULL AND pg_catalog.octet_length(name) BETWEEN 1 AND 128
        AND name COLLATE "C" !~ '[^A-Za-z0-9._-]' AND pg_catalog.left(name,1) COLLATE "C" ~ '[A-Za-z0-9]'
        AND format IS NOT NULL AND format COLLATE "C" IN ('pdf','docx')
        AND policy IS NOT NULL AND policy COLLATE "C"='pdf_docx_v1';
END; $$;
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.case_stage_values_canonical(value %1$I.case_stage_revisions)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE issued BIGINT[]; received BIGINT[];
BEGIN
    IF value.change_kind IS NULL OR value.stage IS NULL OR value.stage COLLATE "C" NOT IN ('investigation','intermediate','trial')
        OR NOT %1$I.case_stage_support_valid(value.support_id,value.support_version,value.support_digest,value.support_name,value.support_format,value.support_policy)
        THEN RETURN FALSE; END IF;
    issued := %1$I.case_stage_time_bounds(value.act_precision,value.act_date,value.act_seconds,value.act_nanoseconds,value.act_offset_seconds);
    IF issued IS NULL THEN RETURN FALSE; END IF;
    IF value.note IS NOT NULL AND NOT %1$I.case_administration_text_valid(value.note,1000,TRUE) THEN RETURN FALSE; END IF;
    IF value.change_kind COLLATE "C"='adoption' THEN
        IF value.from_stage IS NOT NULL OR value.note IS NOT NULL
            OR NOT %1$I.case_administration_text_valid(value.reason,1000,TRUE) THEN RETURN FALSE; END IF;
    ELSIF value.change_kind COLLATE "C"='to_intermediate' THEN
        IF value.stage COLLATE "C"<>'intermediate' OR value.from_stage IS DISTINCT FROM 'investigation'
            OR value.reason IS NOT NULL THEN RETURN FALSE; END IF;
    ELSIF value.change_kind COLLATE "C"='to_trial' THEN
        IF value.stage COLLATE "C"<>'trial' OR value.from_stage IS DISTINCT FROM 'intermediate'
            OR value.reason IS NOT NULL OR NOT %1$I.case_administration_text_valid(value.receiving_court,200,FALSE)
            OR (value.receipt_reference IS NOT NULL AND NOT %1$I.case_administration_text_valid(value.receipt_reference,200,FALSE))
            THEN RETURN FALSE; END IF;
        received := %1$I.case_stage_time_bounds(value.received_precision,value.received_date,value.received_seconds,value.received_nanoseconds,value.received_offset_seconds);
        IF received IS NULL OR (issued[1],issued[2])>(received[3],received[4]) THEN RETURN FALSE; END IF;
        IF pg_catalog.num_nonnulls(value.receipt_id,value.receipt_version,value.receipt_digest,value.receipt_name,value.receipt_format,value.receipt_policy)>0 THEN
            IF NOT %1$I.case_stage_support_valid(value.receipt_id,value.receipt_version,value.receipt_digest,value.receipt_name,value.receipt_format,value.receipt_policy)
                OR (value.receipt_id=value.support_id AND value.receipt_version=value.support_version
                    AND (value.receipt_digest IS DISTINCT FROM value.support_digest
                        OR value.receipt_name IS DISTINCT FROM value.support_name
                        OR value.receipt_format IS DISTINCT FROM value.support_format
                        OR value.receipt_policy IS DISTINCT FROM value.support_policy)) THEN RETURN FALSE; END IF;
        END IF;
        RETURN TRUE;
    ELSE RETURN FALSE;
    END IF;
    RETURN pg_catalog.num_nonnulls(value.received_precision,value.received_date,value.received_seconds,
        value.received_nanoseconds,value.received_offset_seconds,value.receiving_court,value.receipt_reference,
        value.receipt_id,value.receipt_version,value.receipt_digest,value.receipt_name,value.receipt_format,value.receipt_policy)=0;
END; $$;
$definition$, current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.case_stage_values_bytes(value %1$I.case_stage_revisions)
RETURNS BYTEA LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE result BYTEA := pg_catalog.convert_to('CSTG1','UTF8'); encoded BYTEA; item TEXT;
BEGIN
    IF NOT %1$I.case_stage_values_canonical(value) THEN
        RAISE EXCEPTION 'stage values must be canonical' USING ERRCODE='23514';
    END IF;
    result := result || CASE value.change_kind WHEN 'adoption' THEN pg_catalog.decode('00','hex')
        WHEN 'to_intermediate' THEN pg_catalog.decode('01','hex') ELSE pg_catalog.decode('02','hex') END;
    IF value.change_kind='adoption' THEN
        result := result || CASE value.stage WHEN 'investigation' THEN pg_catalog.decode('00','hex')
            WHEN 'intermediate' THEN pg_catalog.decode('01','hex') ELSE pg_catalog.decode('02','hex') END;
    END IF;
    result := result || %1$I.case_stage_time_bytes(value.act_precision,value.act_date,value.act_seconds,value.act_nanoseconds,value.act_offset_seconds);
    IF value.change_kind='adoption' THEN
        encoded := pg_catalog.convert_to(value.reason,'UTF8');
        result := result || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
    END IF;
    result := result || pg_catalog.uuid_send(value.support_id)
        || pg_catalog.substring(pg_catalog.int8send(value.support_version),5,4) || value.support_digest;
    IF value.change_kind='adoption' THEN RETURN result; END IF;
    IF value.change_kind='to_trial' THEN
        result := result || %1$I.case_stage_time_bytes(value.received_precision,value.received_date,value.received_seconds,value.received_nanoseconds,value.received_offset_seconds);
        encoded := pg_catalog.convert_to(value.receiving_court,'UTF8');
        result := result || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
        IF value.receipt_reference IS NULL THEN result := result || pg_catalog.decode('00','hex');
        ELSE
            encoded := pg_catalog.convert_to(value.receipt_reference,'UTF8');
            result := result || pg_catalog.decode('01','hex') || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
        END IF;
        IF value.receipt_id IS NULL THEN result := result || pg_catalog.decode('00','hex');
        ELSE result := result || pg_catalog.decode('01','hex') || pg_catalog.uuid_send(value.receipt_id)
            || pg_catalog.substring(pg_catalog.int8send(value.receipt_version),5,4) || value.receipt_digest;
        END IF;
    END IF;
    IF value.note IS NULL THEN RETURN result || pg_catalog.decode('00','hex'); END IF;
    encoded := pg_catalog.convert_to(value.note,'UTF8');
    RETURN result || pg_catalog.decode('01','hex') || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
END; $$;
$definition$, current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.case_stage_recording_valid(value %1$I.case_stage_revisions)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE issued BIGINT[]; received BIGINT[];
BEGIN
    issued := %1$I.case_stage_time_bounds(value.act_precision,value.act_date,value.act_seconds,value.act_nanoseconds,value.act_offset_seconds);
    IF issued IS NULL OR value.recorded_at_seconds IS NULL OR value.recorded_at_nanoseconds IS NULL
        OR (issued[1],issued[2])>(value.recorded_at_seconds,value.recorded_at_nanoseconds) THEN RETURN FALSE; END IF;
    IF value.change_kind='to_trial' THEN
        received := %1$I.case_stage_time_bounds(value.received_precision,value.received_date,value.received_seconds,value.received_nanoseconds,value.received_offset_seconds);
        RETURN received IS NOT NULL AND (received[1],received[2])<=(value.recorded_at_seconds,value.recorded_at_nanoseconds);
    END IF;
    RETURN TRUE;
END; $$;
$definition$, current_schema());
END; $install$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='case_stage_revisions'::regclass AND conname='case_stage_canonical') THEN
        ALTER TABLE case_stage_revisions ADD CONSTRAINT case_stage_canonical CHECK(case_stage_values_canonical(case_stage_revisions));
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='case_stage_revisions'::regclass AND conname='case_stage_digest') THEN
        ALTER TABLE case_stage_revisions ADD CONSTRAINT case_stage_digest CHECK(values_digest=pg_catalog.sha256(case_stage_values_bytes(case_stage_revisions)));
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='case_stage_revisions'::regclass AND conname='case_stage_recorded_after_acts') THEN
        ALTER TABLE case_stage_revisions ADD CONSTRAINT case_stage_recorded_after_acts CHECK(case_stage_recording_valid(case_stage_revisions));
    END IF;
END; $$;
REVOKE ALL ON FUNCTION case_stage_support_valid(UUID,BIGINT,BYTEA,TEXT,TEXT,TEXT),
    case_stage_values_canonical(case_stage_revisions),case_stage_values_bytes(case_stage_revisions),
    case_stage_recording_valid(case_stage_revisions) FROM PUBLIC;
