CREATE OR REPLACE FUNCTION preserve_resource_activity_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'resource activity history is immutable' USING ERRCODE='23514';
END; $$;
CREATE OR REPLACE FUNCTION lock_resource_activity_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'association writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_resource_activity_sequence() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; administration RECORD; actor RECORD; baseline RECORD; head RECORD;
BEGIN
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.recorded_by_email
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by FOR SHARE))) THEN
        RAISE EXCEPTION 'association actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS NULL THEN
        SELECT * INTO baseline FROM %1$I.cases WHERE id=NEW.case_id;
        IF baseline.id IS NULL OR baseline.required_initial_revision IS NOT NULL
            OR NEW.recorded_administration_revision IS NOT NULL OR NEW.recorded_administration_digest IS NOT NULL
            OR NEW.recorded_administration_title IS DISTINCT FROM baseline.title
            OR NEW.recorded_administration_reference IS DISTINCT FROM baseline.reference THEN
            RAISE EXCEPTION 'association baseline differs' USING ERRCODE='23514';
        END IF;
    ELSIF administration.revision IS DISTINCT FROM NEW.recorded_administration_revision
        OR administration.values_digest IS DISTINCT FROM NEW.recorded_administration_digest
        OR administration.administrative_status IS DISTINCT FROM 'active' THEN
        RAISE EXCEPTION 'association requires the current active administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO head FROM %1$I.case_procedural_resource_revisions WHERE resource_id=NEW.resource_id AND case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF head.revision IS NULL OR head.revision IS DISTINCT FROM NEW.recorded_resource_revision
        OR head.capture_digest IS DISTINCT FROM NEW.recorded_resource_capture_digest
        OR (NEW.action='link' AND head.status<>'active') THEN
        RAISE EXCEPTION 'association resource head differs' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.case_procedural_resource_revisions r WHERE r.resource_id=NEW.resource_id AND r.case_id=NEW.case_id
        AND r.revision=NEW.resource_revision AND r.capture_digest=NEW.resource_capture_digest)
        OR (NEW.act_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM %1$I.case_procedural_resource_revisions r
            WHERE r.resource_id=NEW.resource_id AND r.case_id=NEW.case_id AND r.revision=NEW.act_resource_revision
                AND r.act_id=NEW.act_id AND r.act_revision=NEW.act_revision AND r.capture_digest=NEW.act_capture_digest))
        OR (NEW.hearing_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM %1$I.case_hearing_revisions h
            WHERE h.hearing_id=NEW.hearing_id AND h.case_id=NEW.case_id AND h.revision=NEW.hearing_revision
                AND h.submission_digest=NEW.hearing_submission_digest))
        OR (NEW.deadline_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM %1$I.case_deadline_revisions d
            WHERE d.deadline_id=NEW.deadline_id AND d.case_id=NEW.case_id AND d.revision=NEW.deadline_revision
                AND d.capture_digest=NEW.deadline_capture_digest)) THEN
        RAISE EXCEPTION 'association exact source capture differs' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_resource_activity_association_revisions WHERE association_id=NEW.association_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM coalesce(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'link')
        OR (previous.revision IS NOT NULL AND (NEW.action<>'unlink' OR previous.status<>'linked'
            OR NEW.case_id IS DISTINCT FROM previous.case_id OR NEW.resource_id IS DISTINCT FROM previous.resource_id
            OR NEW.previous_capture_digest IS DISTINCT FROM previous.capture_digest
            OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(previous.recorded_at_seconds,previous.recorded_at_nanoseconds)
            OR ROW(NEW.resource_revision,NEW.resource_capture_digest,NEW.act_id,NEW.act_revision,NEW.act_resource_revision,NEW.act_capture_digest,
                NEW.target_kind,NEW.hearing_id,NEW.hearing_revision,NEW.hearing_submission_digest,NEW.deadline_id,NEW.deadline_revision,NEW.deadline_capture_digest,NEW.selection_canonical)
                IS DISTINCT FROM ROW(previous.resource_revision,previous.resource_capture_digest,previous.act_id,previous.act_revision,previous.act_resource_revision,previous.act_capture_digest,
                previous.target_kind,previous.hearing_id,previous.hearing_revision,previous.hearing_submission_digest,previous.deadline_id,previous.deadline_revision,previous.deadline_capture_digest,previous.selection_canonical))) THEN
        RAISE EXCEPTION 'association revision does not follow its predecessor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_resource_activity_associations','case_resource_activity_association_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='resource_activity_immutable') THEN
            EXECUTE format('CREATE TRIGGER resource_activity_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_resource_activity_history()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='resource_activity_lock') THEN
            EXECUTE format('CREATE TRIGGER resource_activity_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION lock_resource_activity_history()',tab);
        END IF;
    END LOOP;
END; $$;
DROP TRIGGER IF EXISTS resource_activity_sequence ON case_resource_activity_association_revisions;
CREATE TRIGGER resource_activity_sequence BEFORE INSERT ON case_resource_activity_association_revisions FOR EACH ROW EXECUTE FUNCTION enforce_resource_activity_sequence();
REVOKE ALL ON FUNCTION preserve_resource_activity_history(),lock_resource_activity_history(),enforce_resource_activity_sequence() FROM PUBLIC;
