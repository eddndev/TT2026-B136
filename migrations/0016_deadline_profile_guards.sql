-- Immutable revisions share the audited write lock and recheck current authority.
CREATE OR REPLACE FUNCTION preserve_deadline_profile_history()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'deadline profile history is immutable' USING ERRCODE='23514';
END; $$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['deadline_profiles','deadline_profile_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='deadline_profile_immutable') THEN
            EXECUTE format('CREATE TRIGGER deadline_profile_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_deadline_profile_history()',tab);
        END IF;
    END LOOP;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_deadline_profile_sequence()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; actor RECORD; root RECORD; initial JSONB; value JSONB;
    current_status TEXT; required_revision BIGINT;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'deadline profile writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.role IS DISTINCT FROM 'owner'
        OR actor.email COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C" THEN
        RAISE EXCEPTION 'deadline profile actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO root FROM %1$I.deadline_profiles WHERE id=NEW.profile_id;
    IF root.id IS NULL THEN
        RAISE EXCEPTION 'deadline profile root must exist' USING ERRCODE='23503';
    END IF;
    IF root.case_id IS NOT NULL THEN
        SELECT required_initial_revision INTO required_revision FROM %1$I.cases WHERE id=root.case_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'deadline profile case must exist' USING ERRCODE='23503';
        END IF;
        SELECT administrative_status INTO current_status FROM %1$I.case_administration_revisions
            WHERE case_id=root.case_id ORDER BY revision DESC LIMIT 1;
        IF current_status IS DISTINCT FROM 'active' AND (current_status IS NOT NULL OR required_revision IS NOT NULL) THEN
            RAISE EXCEPTION 'deadline profile case must currently be active' USING ERRCODE='23514';
        END IF;
    END IF;
    value:=%1$I.deadline_profile_definition(NEW.definition_canonical);
    IF (value->'scope'->>'case_id')::uuid IS DISTINCT FROM root.case_id THEN
        RAISE EXCEPTION 'deadline profile definition changes the root scope' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.deadline_profile_revisions
        WHERE profile_id=NEW.profile_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM coalesce(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'publish')
        OR (previous.revision IS NOT NULL AND (NEW.action='publish' OR previous.status<>'published')) THEN
        RAISE EXCEPTION 'deadline profile revisions must follow a published predecessor' USING ERRCODE='23514';
    END IF;
    IF previous.revision IS NOT NULL THEN
        SELECT definition_view->'scope' INTO initial FROM %1$I.deadline_profile_revisions
            WHERE profile_id=NEW.profile_id AND revision=1;
        IF initial IS DISTINCT FROM value->'scope' THEN
            RAISE EXCEPTION 'deadline profile scope must equal its first revision' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.action='retire' AND ROW(NEW.definition_canonical,NEW.definition_digest,NEW.algorithm)
        IS DISTINCT FROM ROW(previous.definition_canonical,previous.definition_digest,previous.algorithm) THEN
        RAISE EXCEPTION 'deadline profile retirement preserves exact definition and algorithm' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS deadline_profile_sequence ON deadline_profile_revisions;
CREATE TRIGGER deadline_profile_sequence BEFORE INSERT ON deadline_profile_revisions
    FOR EACH ROW EXECUTE FUNCTION enforce_deadline_profile_sequence();
REVOKE ALL ON FUNCTION preserve_deadline_profile_history(),enforce_deadline_profile_sequence() FROM PUBLIC;
