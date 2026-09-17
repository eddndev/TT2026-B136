-- Profile changes extend the existing serialized source-change stream.
ALTER TABLE deadline_source_events
    ADD COLUMN IF NOT EXISTS profile_id UUID GENERATED ALWAYS AS
        (CASE WHEN source_kind='profile' THEN source_id ELSE NULL END) STORED;
ALTER TABLE deadline_source_events
    DROP CONSTRAINT IF EXISTS deadline_source_kind,
    ADD CONSTRAINT deadline_source_kind CHECK(source_kind COLLATE "C" IN
        ('resolution','notification','hearing_result','calendar','profile')),
    DROP CONSTRAINT IF EXISTS deadline_source_case_shape,
    ADD CONSTRAINT deadline_source_case_shape CHECK(
        source_kind='profile' OR (source_kind='calendar')=(case_id IS NULL));
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='deadline_source_events'::regclass
        AND conname='deadline_source_profile_revision') THEN
        ALTER TABLE deadline_source_events ADD CONSTRAINT deadline_source_profile_revision
            FOREIGN KEY(profile_id,revision) REFERENCES deadline_profile_revisions(profile_id,revision);
    END IF;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_deadline_source_event()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE source_operation UUID; source_case UUID;
BEGIN
    IF NEW.sequence IS NOT NULL OR current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'deadline event sequence requires serialized allocation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    CASE NEW.source_kind
    WHEN 'resolution','notification' THEN
        SELECT operation_id INTO source_operation FROM %1$I.case_procedural_fact_revisions
            WHERE family=NEW.source_kind AND id=NEW.source_id AND revision=NEW.revision;
    WHEN 'hearing_result' THEN
        SELECT operation_id INTO source_operation FROM %1$I.case_hearing_result_revisions
            WHERE result_id=NEW.source_id AND revision=NEW.revision;
    WHEN 'calendar' THEN
        SELECT operation_id INTO source_operation FROM %1$I.judicial_calendar_revisions
            WHERE calendar_id=NEW.source_id AND revision=NEW.revision;
    WHEN 'profile' THEN
        SELECT r.operation_id,p.case_id INTO source_operation,source_case
            FROM %1$I.deadline_profile_revisions r JOIN %1$I.deadline_profiles p ON p.id=r.profile_id
            WHERE r.profile_id=NEW.source_id AND r.revision=NEW.revision;
    ELSE
        RAISE EXCEPTION 'unknown deadline source family' USING ERRCODE='23514';
    END CASE;
    IF source_operation IS NULL THEN
        RAISE EXCEPTION 'deadline source revision does not exist' USING ERRCODE='23503';
    END IF;
    IF NEW.operation_id IS DISTINCT FROM source_operation THEN
        RAISE EXCEPTION 'deadline source operation does not match its revision' USING ERRCODE='23514';
    END IF;
    IF NEW.source_kind='profile' AND NEW.case_id IS DISTINCT FROM source_case THEN
        RAISE EXCEPTION 'deadline profile source scope does not match its root' USING ERRCODE='23514';
    END IF;
    NEW.sequence:=nextval(pg_get_serial_sequence(TG_RELID::regclass::text,'sequence')::regclass);
    RETURN NEW;
END; $$;
CREATE OR REPLACE FUNCTION %1$I.emit_deadline_source_event()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE source_case UUID;
BEGIN
    CASE TG_TABLE_NAME
    WHEN 'case_procedural_fact_revisions' THEN
        INSERT INTO %1$I.deadline_source_events(source_kind,source_id,revision,case_id,operation_id)
            VALUES(NEW.family,NEW.id,NEW.revision,NEW.case_id,NEW.operation_id);
    WHEN 'case_hearing_result_revisions' THEN
        INSERT INTO %1$I.deadline_source_events(source_kind,source_id,revision,case_id,hearing_id,operation_id)
            VALUES('hearing_result',NEW.result_id,NEW.revision,NEW.case_id,NEW.hearing_id,NEW.operation_id);
    WHEN 'judicial_calendar_revisions' THEN
        INSERT INTO %1$I.deadline_source_events(source_kind,source_id,revision,operation_id)
            VALUES('calendar',NEW.calendar_id,NEW.revision,NEW.operation_id);
    WHEN 'deadline_profile_revisions' THEN
        SELECT case_id INTO source_case FROM %1$I.deadline_profiles WHERE id=NEW.profile_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'deadline profile source root does not exist' USING ERRCODE='23503';
        END IF;
        INSERT INTO %1$I.deadline_source_events(source_kind,source_id,revision,case_id,operation_id)
            VALUES('profile',NEW.profile_id,NEW.revision,source_case,NEW.operation_id);
    ELSE
        RAISE EXCEPTION 'unsupported deadline source table' USING ERRCODE='23514';
    END CASE;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='deadline_profile_revisions'::regclass
        AND tgname='deadline_source_emit') THEN
        CREATE TRIGGER deadline_source_emit AFTER INSERT ON deadline_profile_revisions
            FOR EACH ROW EXECUTE FUNCTION emit_deadline_source_event();
    END IF;
END; $$;
REVOKE ALL ON FUNCTION validate_deadline_source_event(),emit_deadline_source_event() FROM PUBLIC;
