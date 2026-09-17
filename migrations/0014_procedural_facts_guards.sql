-- Every declaration writer shares the audit lock and rechecks current authority.
CREATE OR REPLACE FUNCTION preserve_procedural_fact_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'procedural fact history is immutable' USING ERRCODE='23514';
END; $$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_procedural_facts','case_procedural_fact_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='procedural_fact_immutable') THEN
            EXECUTE format('CREATE TRIGGER procedural_fact_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_procedural_fact_history()',tab);
        END IF;
    END LOOP;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_procedural_fact_root() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'fact writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    IF NEW.family='notification' AND NOT EXISTS(SELECT 1 FROM %1$I.case_procedural_fact_revisions
        WHERE family='resolution' AND id=NEW.parent_resolution_id AND case_id=NEW.case_id) THEN
        RAISE EXCEPTION 'fact parent must already exist in the same case' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_procedural_fact_sequence() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; administration RECORD; actor RECORD; root RECORD; baseline RECORD;
    value JSONB; receipt JSONB; sources JSONB; item JSONB; prior JSONB; field TEXT;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'fact writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.recorded_by_email
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by))) THEN
        RAISE EXCEPTION 'fact actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS NULL THEN
        SELECT * INTO baseline FROM %1$I.cases WHERE id=NEW.case_id;
        IF baseline.id IS NULL OR baseline.required_initial_revision IS NOT NULL
            OR NEW.recorded_administration_revision IS NOT NULL OR NEW.recorded_administration_digest IS NOT NULL
            OR NEW.recorded_administration_title IS DISTINCT FROM baseline.title
            OR NEW.recorded_administration_reference IS DISTINCT FROM baseline.reference THEN
            RAISE EXCEPTION 'fact requires the exact unrevised administration' USING ERRCODE='23514';
        END IF;
    ELSIF administration.revision IS DISTINCT FROM NEW.recorded_administration_revision
        OR administration.values_digest IS DISTINCT FROM NEW.recorded_administration_digest
        OR administration.administrative_status IS DISTINCT FROM 'active'
        OR NEW.recorded_administration_title IS NOT NULL OR NEW.recorded_administration_reference IS NOT NULL THEN
        RAISE EXCEPTION 'fact requires the current active administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO root FROM %1$I.case_procedural_facts WHERE family=NEW.family AND id=NEW.id;
    receipt:=%1$I.procedural_fact_submission(NEW.submission_canonical);
    IF root.case_id IS DISTINCT FROM NEW.case_id OR (receipt->>'parent_id')::uuid IS DISTINCT FROM root.parent_resolution_id THEN
        RAISE EXCEPTION 'fact receipt changes its immutable scope or parent' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_procedural_fact_revisions WHERE family=NEW.family AND id=NEW.id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM COALESCE(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'record')
        OR (previous.revision IS NOT NULL AND (NEW.action='record' OR previous.status<>'recorded'
            OR previous.case_id IS DISTINCT FROM NEW.case_id)) THEN
        RAISE EXCEPTION 'fact revisions must follow a recorded predecessor' USING ERRCODE='23514';
    END IF;
    IF NEW.action='withdraw' AND ROW(NEW.values_canonical,NEW.values_digest,NEW.sources_canonical,NEW.sources_digest)
        IS DISTINCT FROM ROW(previous.values_canonical,previous.values_digest,previous.sources_canonical,previous.sources_digest) THEN
        RAISE EXCEPTION 'withdrawal preserves exact values and sources' USING ERRCODE='23514';
    END IF;
    value:=%1$I.procedural_fact_values(NEW.family,NEW.values_canonical);
    sources:=%1$I.procedural_fact_sources(NEW.sources_canonical);
    IF NEW.family='notification' AND (value->'resolution'->>'id')::uuid IS DISTINCT FROM root.parent_resolution_id THEN
        RAISE EXCEPTION 'notification values change its parent' USING ERRCODE='23514';
    END IF;
    IF previous.revision IS NOT NULL THEN
        IF sources->'resolution'->>'id'=previous.sources_view->'resolution'->>'id'
            AND sources->'resolution'->>'revision'=previous.sources_view->'resolution'->>'revision'
            AND sources->'resolution' IS DISTINCT FROM previous.sources_view->'resolution' THEN
            RAISE EXCEPTION 'retained parent source changed' USING ERRCODE='23514';
        END IF;
        FOREACH field IN ARRAY ARRAY['participants','hearing_results','direct_supports'] LOOP
            FOR item IN SELECT jsonb_array_elements(sources->field) LOOP
                FOR prior IN SELECT jsonb_array_elements(previous.sources_view->field) LOOP
                    IF field='hearing_results' THEN
                        IF item->'hearing_id'=prior->'hearing_id' AND item->'result_id'=prior->'result_id' AND item->'revision'=prior->'revision'
                            AND ((item-'agreement_id'-'agreement_text') IS DISTINCT FROM (prior-'agreement_id'-'agreement_text')
                                OR (item->'agreement_id'=prior->'agreement_id' AND item->'agreement_text' IS DISTINCT FROM prior->'agreement_text')) THEN
                            RAISE EXCEPTION 'retained result source changed' USING ERRCODE='23514';
                        END IF;
                    ELSIF item->'id'=prior->'id' AND item->(CASE field WHEN 'participants' THEN 'revision' ELSE 'version' END)
                        =prior->(CASE field WHEN 'participants' THEN 'revision' ELSE 'version' END) AND item IS DISTINCT FROM prior THEN
                        RAISE EXCEPTION 'retained fact source changed' USING ERRCODE='23514';
                    END IF;
                END LOOP;
            END LOOP;
        END LOOP;
    END IF;
    PERFORM %1$I.validate_procedural_fact_sources(NEW.case_id,NEW.family,value,sources);
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS procedural_fact_root_sources ON case_procedural_facts;
CREATE TRIGGER procedural_fact_root_sources BEFORE INSERT ON case_procedural_facts FOR EACH ROW EXECUTE FUNCTION enforce_procedural_fact_root();
DROP TRIGGER IF EXISTS procedural_fact_sequence ON case_procedural_fact_revisions;
CREATE TRIGGER procedural_fact_sequence BEFORE INSERT ON case_procedural_fact_revisions FOR EACH ROW EXECUTE FUNCTION enforce_procedural_fact_sequence();
REVOKE ALL ON FUNCTION preserve_procedural_fact_history(),enforce_procedural_fact_root(),enforce_procedural_fact_sequence() FROM PUBLIC;
