CREATE OR REPLACE FUNCTION preserve_resource_hearing_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'resource hearing history is immutable' USING ERRCODE='23514';
END; $$;
CREATE OR REPLACE FUNCTION lock_resource_hearing_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'resource hearing writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_resource_hearing_sequence() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE administration RECORD; actor RECORD; baseline RECORD; head RECORD;
BEGIN
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.recorded_by_email
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by FOR SHARE))) THEN
        RAISE EXCEPTION 'resource hearing actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS NULL THEN
        SELECT * INTO baseline FROM %1$I.cases WHERE id=NEW.case_id;
        IF baseline.id IS NULL OR baseline.required_initial_revision IS NOT NULL
            OR NEW.recorded_administration_revision IS NOT NULL OR NEW.recorded_administration_digest IS NOT NULL
            OR NEW.recorded_administration_title IS DISTINCT FROM baseline.title
            OR NEW.recorded_administration_reference IS DISTINCT FROM baseline.reference THEN
            RAISE EXCEPTION 'resource hearing baseline differs' USING ERRCODE='23514';
        END IF;
    ELSIF administration.revision IS DISTINCT FROM NEW.recorded_administration_revision
        OR administration.values_digest IS DISTINCT FROM NEW.recorded_administration_digest
        OR administration.administrative_status IS DISTINCT FROM 'active' THEN
        RAISE EXCEPTION 'resource hearing requires the current active administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO head FROM %1$I.case_procedural_resource_revisions WHERE resource_id=NEW.resource_id AND case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF head.revision IS NULL OR head.revision IS DISTINCT FROM NEW.recorded_resource_revision
        OR head.capture_digest IS DISTINCT FROM NEW.recorded_resource_capture_digest OR head.status<>'active'
        OR NEW.resource_revision>head.revision OR NEW.act_resource_revision>head.revision
        OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(head.recorded_at_seconds,head.recorded_at_nanoseconds) THEN
        RAISE EXCEPTION 'resource hearing resource head differs' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.case_procedural_resource_revisions r WHERE r.resource_id=NEW.resource_id AND r.case_id=NEW.case_id
        AND r.revision=NEW.resource_revision AND r.capture_digest=NEW.resource_capture_digest)
        OR (NEW.act_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM %1$I.case_procedural_resource_revisions r
            WHERE r.resource_id=NEW.resource_id AND r.case_id=NEW.case_id AND r.revision=NEW.act_resource_revision
                AND r.act_id=NEW.act_id AND r.act_revision=NEW.act_revision AND r.capture_digest=NEW.act_capture_digest)) THEN
        RAISE EXCEPTION 'resource hearing exact source capture differs' USING ERRCODE='23514';
    END IF;
    IF NEW.revision<>1 OR EXISTS(SELECT 1 FROM %1$I.case_resource_hearing_revisions WHERE hearing_id=NEW.hearing_id) THEN
        RAISE EXCEPTION 'resource hearing creation must be its initial revision' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_resource_hearings','case_resource_hearing_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='resource_hearing_immutable') THEN
            EXECUTE format('CREATE TRIGGER resource_hearing_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_resource_hearing_history()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='resource_hearing_lock') THEN
            EXECUTE format('CREATE TRIGGER resource_hearing_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION lock_resource_hearing_history()',tab);
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_resource_hearing_revisions'::regclass AND tgname='resource_hearing_sequence') THEN
        CREATE TRIGGER resource_hearing_sequence BEFORE INSERT ON case_resource_hearing_revisions FOR EACH ROW EXECUTE FUNCTION enforce_resource_hearing_sequence();
    END IF;
END; $$;
REVOKE ALL ON FUNCTION preserve_resource_hearing_history(),lock_resource_hearing_history(),enforce_resource_hearing_sequence() FROM PUBLIC;
