CREATE OR REPLACE FUNCTION preserve_procedural_resource_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'procedural resource history is immutable' USING ERRCODE='23514';
END; $$;
CREATE OR REPLACE FUNCTION lock_procedural_resource_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'resource writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_procedural_resource_sequence() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; prior_act RECORD; administration RECORD; actor RECORD; baseline RECORD; stage BIGINT;
BEGIN
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.recorded_by_email
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by FOR SHARE))) THEN
        RAISE EXCEPTION 'resource actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS NULL THEN
        SELECT * INTO baseline FROM %1$I.cases WHERE id=NEW.case_id;
        IF baseline.id IS NULL OR baseline.required_initial_revision IS NOT NULL
            OR NEW.recorded_administration_revision IS NOT NULL OR NEW.recorded_administration_digest IS NOT NULL
            OR NEW.recorded_administration_title IS DISTINCT FROM baseline.title
            OR NEW.recorded_administration_reference IS DISTINCT FROM baseline.reference THEN
            RAISE EXCEPTION 'resource baseline differs' USING ERRCODE='23514';
        END IF;
    ELSIF administration.revision IS DISTINCT FROM NEW.recorded_administration_revision
        OR administration.values_digest IS DISTINCT FROM NEW.recorded_administration_digest
        OR administration.administrative_status IS DISTINCT FROM 'active' THEN
        RAISE EXCEPTION 'resource requires the current active administration' USING ERRCODE='23514';
    END IF;
    SELECT max(revision) INTO stage FROM (SELECT revision FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id
        UNION ALL SELECT stage_revision FROM %1$I.case_initial_stage_registrations WHERE case_id=NEW.case_id) s;
    IF stage IS DISTINCT FROM NEW.recorded_stage_revision THEN
        RAISE EXCEPTION 'resource stage observation differs' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_procedural_resource_revisions WHERE resource_id=NEW.resource_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM coalesce(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'register')
        OR (previous.revision IS NOT NULL AND (NEW.action='register' OR NEW.case_id IS DISTINCT FROM previous.case_id
            OR NEW.previous_capture_digest IS DISTINCT FROM previous.capture_digest
            OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(previous.recorded_at_seconds,previous.recorded_at_nanoseconds)
            OR (previous.status='archived' AND NEW.action<>'reactivate') OR (previous.status='active' AND NEW.action='reactivate'))) THEN
        RAISE EXCEPTION 'resource revision does not follow its predecessor' USING ERRCODE='23514';
    END IF;
    IF NEW.action NOT IN ('register','correct') AND ROW(NEW.values_canonical,NEW.values_view,NEW.sources_canonical,NEW.supports_view)
        IS DISTINCT FROM ROW(previous.values_canonical,previous.values_view,previous.sources_canonical,previous.supports_view) THEN
        RAISE EXCEPTION 'resource command changes retained capture' USING ERRCODE='23514';
    END IF;
    IF NEW.act_id IS NOT NULL THEN
        SELECT * INTO prior_act FROM %1$I.case_procedural_resource_revisions WHERE act_id=NEW.act_id ORDER BY act_revision DESC LIMIT 1;
        IF NEW.act_revision IS DISTINCT FROM coalesce(prior_act.act_revision+1,1)
            OR (prior_act.act_revision IS NULL AND (NEW.action<>'record_act' OR NOT EXISTS(
                SELECT 1 FROM %1$I.case_procedural_resource_acts WHERE id=NEW.act_id AND resource_id=NEW.resource_id
                    AND case_id=NEW.case_id AND initial_resource_revision=NEW.revision)))
            OR (prior_act.act_revision IS NOT NULL AND (NEW.action<>'correct_act'
                OR NEW.act_previous_resource_revision IS DISTINCT FROM prior_act.revision
                OR NEW.act_previous_capture_digest IS DISTINCT FROM prior_act.capture_digest)) THEN
            RAISE EXCEPTION 'resource act does not follow its predecessor' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_procedural_resources','case_procedural_resource_acts','case_procedural_resource_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='procedural_resource_immutable') THEN
            EXECUTE format('CREATE TRIGGER procedural_resource_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_procedural_resource_history()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='procedural_resource_lock') THEN
            EXECUTE format('CREATE TRIGGER procedural_resource_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION lock_procedural_resource_history()',tab);
        END IF;
    END LOOP;
END; $$;
DROP TRIGGER IF EXISTS procedural_resource_sequence ON case_procedural_resource_revisions;
CREATE TRIGGER procedural_resource_sequence BEFORE INSERT ON case_procedural_resource_revisions FOR EACH ROW EXECUTE FUNCTION enforce_procedural_resource_sequence();
REVOKE ALL ON FUNCTION preserve_procedural_resource_history(),lock_procedural_resource_history(),enforce_procedural_resource_sequence() FROM PUBLIC;
