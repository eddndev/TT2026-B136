-- Declared stage history is separate from the unchanged initial registration.
CREATE OR REPLACE FUNCTION case_stage_time_bounds(
    time_precision TEXT, local_date DATE, seconds BIGINT, nanos INTEGER, offset_seconds INTEGER)
RETURNS BIGINT[] LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE lower_second BIGINT; upper_second BIGINT; local_second BIGINT;
BEGIN
    IF time_precision IS NULL OR offset_seconds IS NULL OR offset_seconds NOT BETWEEN -50400 AND 50400
        OR offset_seconds % 60<>0 THEN RETURN NULL; END IF;
    IF time_precision COLLATE "C"='date' THEN
        IF local_date IS NULL OR NOT pg_catalog.isfinite(local_date) OR seconds IS NOT NULL OR nanos IS NOT NULL
            OR local_date NOT BETWEEN DATE '0001-01-01' AND DATE '9999-12-31' THEN RETURN NULL; END IF;
        lower_second := (local_date-DATE '1970-01-01')::BIGINT*86400-offset_seconds;
        upper_second := lower_second+86399;
        IF lower_second < -62135596800 OR upper_second > 253402300799 THEN RETURN NULL; END IF;
        RETURN ARRAY[lower_second,0,upper_second,999999999];
    ELSIF time_precision COLLATE "C"='instant' THEN
        IF local_date IS NOT NULL OR seconds IS NULL OR nanos IS NULL
            OR seconds NOT BETWEEN -62135596800 AND 253402300799
            OR nanos NOT BETWEEN 0 AND 999999999 THEN RETURN NULL; END IF;
        local_second := seconds+offset_seconds;
        IF local_second NOT BETWEEN -62135596800 AND 253402300799 THEN RETURN NULL; END IF;
        RETURN ARRAY[seconds,nanos::BIGINT,seconds,nanos::BIGINT];
    END IF;
    RETURN NULL;
END; $$;
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.case_stage_time_bytes(
    time_precision TEXT, local_date DATE, seconds BIGINT, nanos INTEGER, offset_seconds INTEGER)
RETURNS BYTEA LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE result BYTEA;
BEGIN
    IF %1$I.case_stage_time_bounds(time_precision,local_date,seconds,nanos,offset_seconds) IS NULL THEN
        RAISE EXCEPTION 'declared stage time is not canonical' USING ERRCODE='23514';
    END IF;
    IF time_precision COLLATE "C"='date' THEN
        result := pg_catalog.decode('00','hex')
            || pg_catalog.int2send(EXTRACT(YEAR FROM local_date)::SMALLINT)
            || pg_catalog.substring(pg_catalog.int4send(EXTRACT(MONTH FROM local_date)::INTEGER),4,1)
            || pg_catalog.substring(pg_catalog.int4send(EXTRACT(DAY FROM local_date)::INTEGER),4,1);
    ELSE result := pg_catalog.decode('01','hex') || pg_catalog.int8send(seconds) || pg_catalog.int4send(nanos);
    END IF;
    RETURN result || pg_catalog.int4send(offset_seconds);
END; $$;
$definition$, current_schema());
END; $install$;
CREATE TABLE IF NOT EXISTS case_stage_revisions (
    case_id UUID NOT NULL, revision BIGINT NOT NULL, change_kind TEXT NOT NULL,
    from_stage TEXT, stage TEXT NOT NULL, administration_revision BIGINT NOT NULL,
    act_precision TEXT NOT NULL, act_date DATE, act_seconds BIGINT, act_nanoseconds INTEGER,
    act_offset_seconds INTEGER NOT NULL,
    received_precision TEXT, received_date DATE, received_seconds BIGINT,
    received_nanoseconds INTEGER, received_offset_seconds INTEGER,
    reason TEXT, note TEXT, receiving_court TEXT, receipt_reference TEXT,
    support_id UUID NOT NULL, support_version BIGINT NOT NULL, support_digest BYTEA NOT NULL,
    support_name TEXT NOT NULL, support_format TEXT NOT NULL, support_policy TEXT NOT NULL,
    receipt_id UUID, receipt_version BIGINT, receipt_digest BYTEA,
    receipt_name TEXT, receipt_format TEXT, receipt_policy TEXT,
    values_digest BYTEA NOT NULL, recorded_at_seconds BIGINT NOT NULL,
    recorded_at_nanoseconds INTEGER NOT NULL, recorded_by UUID NOT NULL, recorded_by_email TEXT NOT NULL,
    PRIMARY KEY(case_id,revision),
    CONSTRAINT case_stage_root_fk FOREIGN KEY(case_id) REFERENCES cases(id),
    CONSTRAINT case_stage_administration_fk FOREIGN KEY(case_id,administration_revision)
        REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT case_stage_actor_fk FOREIGN KEY(recorded_by) REFERENCES users(id),
    CONSTRAINT case_stage_support_fk FOREIGN KEY(support_id,support_version) REFERENCES documents(id,version),
    CONSTRAINT case_stage_receipt_fk FOREIGN KEY(receipt_id,receipt_version) REFERENCES documents(id,version),
    CONSTRAINT case_stage_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    CONSTRAINT case_stage_recording_range CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799
        AND recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    CONSTRAINT case_stage_actor_email CHECK(case_administration_text_valid(recorded_by_email,254,FALSE))
);
REVOKE ALL ON case_stage_revisions FROM PUBLIC;
REVOKE ALL ON FUNCTION case_stage_time_bounds(TEXT,DATE,BIGINT,INTEGER,INTEGER),
    case_stage_time_bytes(TEXT,DATE,BIGINT,INTEGER,INTEGER) FROM PUBLIC;
