-- Source changes are durable under the same lock as their audited revisions.
CREATE TABLE IF NOT EXISTS deadline_source_events (
    sequence BIGINT CONSTRAINT deadline_source_event_primary PRIMARY KEY,
    source_kind TEXT NOT NULL CONSTRAINT deadline_source_kind CHECK(source_kind COLLATE "C" IN ('resolution','notification','hearing_result','calendar')),
    source_id UUID NOT NULL,
    revision BIGINT NOT NULL CONSTRAINT deadline_source_revision CHECK(revision BETWEEN 1 AND 4294967295),
    case_id UUID,
    hearing_id UUID,
    operation_id UUID NOT NULL,
    fact_family TEXT GENERATED ALWAYS AS (CASE WHEN source_kind IN ('resolution','notification') THEN source_kind ELSE NULL END) STORED,
    hearing_result_id UUID GENERATED ALWAYS AS (CASE WHEN source_kind='hearing_result' THEN source_id ELSE NULL END) STORED,
    calendar_id UUID GENERATED ALWAYS AS (CASE WHEN source_kind='calendar' THEN source_id ELSE NULL END) STORED,
    CONSTRAINT deadline_source_sequence_positive CHECK(sequence>0),
    CONSTRAINT deadline_source_case_shape CHECK((source_kind='calendar')=(case_id IS NULL)),
    CONSTRAINT deadline_source_hearing_shape CHECK((source_kind='hearing_result')=(hearing_id IS NOT NULL)),
    CONSTRAINT deadline_source_revision_unique UNIQUE(source_kind,source_id,revision),
    CONSTRAINT deadline_source_fact_revision FOREIGN KEY(fact_family,source_id,revision)
        REFERENCES case_procedural_fact_revisions(family,id,revision),
    CONSTRAINT deadline_source_fact_scope FOREIGN KEY(fact_family,source_id,case_id)
        REFERENCES case_procedural_facts(family,id,case_id),
    CONSTRAINT deadline_source_hearing_revision FOREIGN KEY(hearing_result_id,revision)
        REFERENCES case_hearing_result_revisions(result_id,revision),
    CONSTRAINT deadline_source_hearing_scope FOREIGN KEY(hearing_result_id,case_id,hearing_id)
        REFERENCES case_hearing_results(id,case_id,hearing_id),
    CONSTRAINT deadline_source_calendar_revision FOREIGN KEY(calendar_id,revision)
        REFERENCES judicial_calendar_revisions(calendar_id,revision)
);
CREATE SEQUENCE IF NOT EXISTS deadline_source_events_sequence AS BIGINT
    START WITH 1 INCREMENT BY 1 MINVALUE 1 NO MAXVALUE CACHE 1 NO CYCLE
    OWNED BY deadline_source_events.sequence;
CREATE OR REPLACE FUNCTION preserve_deadline_source_events()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'deadline source events are immutable' USING ERRCODE='23514';
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_deadline_source_event()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE source_operation UUID;
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
    ELSE
        RAISE EXCEPTION 'unknown deadline source family' USING ERRCODE='23514';
    END CASE;
    IF source_operation IS NULL THEN
        RAISE EXCEPTION 'deadline source revision does not exist' USING ERRCODE='23503';
    END IF;
    IF NEW.operation_id IS DISTINCT FROM source_operation THEN
        RAISE EXCEPTION 'deadline source operation does not match its revision' USING ERRCODE='23514';
    END IF;
    NEW.sequence:=nextval(pg_get_serial_sequence(TG_RELID::regclass::text,'sequence')::regclass);
    RETURN NEW;
END; $$;
CREATE OR REPLACE FUNCTION %1$I.emit_deadline_source_event()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
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
    ELSE
        RAISE EXCEPTION 'unsupported deadline source table' USING ERRCODE='23514';
    END CASE;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_procedural_fact_revisions','case_hearing_result_revisions','judicial_calendar_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='deadline_source_emit') THEN
            EXECUTE format('CREATE TRIGGER deadline_source_emit AFTER INSERT ON %I FOR EACH ROW EXECUTE FUNCTION emit_deadline_source_event()',tab);
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='deadline_source_events'::regclass AND tgname='deadline_source_insert') THEN
        CREATE TRIGGER deadline_source_insert BEFORE INSERT ON deadline_source_events
            FOR EACH ROW EXECUTE FUNCTION validate_deadline_source_event();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='deadline_source_events'::regclass AND tgname='deadline_source_immutable') THEN
        CREATE TRIGGER deadline_source_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON deadline_source_events
            FOR EACH STATEMENT EXECUTE FUNCTION preserve_deadline_source_events();
    END IF;
END; $$;
REVOKE ALL ON deadline_source_events FROM PUBLIC;
REVOKE ALL ON SEQUENCE deadline_source_events_sequence FROM PUBLIC;
REVOKE ALL ON FUNCTION preserve_deadline_source_events(),validate_deadline_source_event(),emit_deadline_source_event() FROM PUBLIC;
