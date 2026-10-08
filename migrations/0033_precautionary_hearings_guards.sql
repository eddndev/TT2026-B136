CREATE OR REPLACE FUNCTION preserve_precautionary_hearing_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'precautionary hearing history is immutable' USING ERRCODE='23514';
END; $$;
CREATE OR REPLACE FUNCTION lock_precautionary_hearing_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'precautionary hearing writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_precautionary_hearing_sequence() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; administration RECORD; actor RECORD; current_stage RECORD;
    value JSONB; selected JSONB; participant RECORD; source_count BIGINT;
    previous_participant UUID; selected_id UUID; source_seconds BIGINT; source_nanos INTEGER;
    audit RECORD; recorded TEXT; marker TEXT;
BEGIN
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE
        OR actor.email COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C"
        OR actor.role COLLATE "C" IS DISTINCT FROM NEW.recorded_by_role COLLATE "C"
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by FOR SHARE))) THEN
        RAISE EXCEPTION 'precautionary hearing actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS DISTINCT FROM NEW.observed_administration_revision
        OR administration.administrative_status IS DISTINCT FROM 'active' OR administration.nuc IS NULL THEN
        RAISE EXCEPTION 'precautionary hearing requires current active complete administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO current_stage FROM (
        SELECT revision,administration_revision,recorded_at_seconds,recorded_at_nanoseconds
            FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id
        UNION ALL SELECT stage_revision,administration_revision,NULL::bigint,NULL::integer
            FROM %1$I.case_initial_stage_registrations WHERE case_id=NEW.case_id
    ) stages ORDER BY revision DESC LIMIT 1;
    SELECT count(*) INTO source_count FROM (
        SELECT revision FROM %1$I.case_stage_revisions
            WHERE case_id=NEW.case_id AND revision=NEW.observed_stage_revision
        UNION ALL SELECT stage_revision FROM %1$I.case_initial_stage_registrations
            WHERE case_id=NEW.case_id AND stage_revision=NEW.observed_stage_revision
    ) stages;
    IF source_count<>1 OR current_stage.revision IS DISTINCT FROM NEW.observed_stage_revision
        OR current_stage.administration_revision>NEW.observed_administration_revision
        OR NOT EXISTS(SELECT 1 FROM %1$I.case_administration_revisions
            WHERE case_id=NEW.case_id AND revision=current_stage.administration_revision
                AND administrative_status='active' AND nuc IS NOT NULL)
        OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)
            <ROW(current_stage.recorded_at_seconds,current_stage.recorded_at_nanoseconds) THEN
        RAISE EXCEPTION 'precautionary hearing requires the exact current stage' USING ERRCODE='23514';
    END IF;
    source_seconds:=extract(epoch FROM regexp_replace(administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
    source_nanos:=rpad(coalesce(substring(administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    IF ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
        RAISE EXCEPTION 'precautionary hearing predates observed administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_precautionary_hearing_revisions
        WHERE hearing_id=NEW.hearing_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM coalesce(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'schedule')
        OR (previous.revision IS NOT NULL AND (NEW.action='schedule' OR previous.action='cancel'
            OR previous.case_id IS DISTINCT FROM NEW.case_id
            OR previous.capture_digest IS DISTINCT FROM NEW.previous_capture_digest
            OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)
                <ROW(previous.recorded_at_seconds,previous.recorded_at_nanoseconds))) THEN
        RAISE EXCEPTION 'precautionary hearing requires its exact scheduled predecessor' USING ERRCODE='23514';
    END IF;
    recorded:=to_char(to_timestamp(NEW.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
        ||CASE WHEN NEW.recorded_at_nanoseconds=0 THEN 'Z'
            ELSE '.'||rtrim(lpad(NEW.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END;
    marker:='ph1:case:'||NEW.case_id::text||':hearing:'||NEW.hearing_id::text
        ||':operation:'||NEW.operation_id::text||':revision:'||NEW.revision::text
        ||':submission:'||encode(NEW.submission_digest,'hex')||':review:'||encode(NEW.review_digest,'hex')
        ||':capture:'||encode(NEW.capture_digest,'hex');
    SELECT * INTO audit FROM %1$I.audit_events WHERE sequence=NEW.audit_sequence;
    IF audit.actor COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C"
        OR audit.action COLLATE "C" IS DISTINCT FROM ('precautionary_hearing.'||NEW.action) COLLATE "C"
        OR audit.resource COLLATE "C" IS DISTINCT FROM marker COLLATE "C"
        OR audit.timestamp COLLATE "C" IS DISTINCT FROM recorded COLLATE "C" THEN
        RAISE EXCEPTION 'precautionary hearing audit marker differs' USING ERRCODE='23514';
    END IF;
    IF NEW.action='cancel' THEN RETURN NEW; END IF;
    value:=NEW.values_view;
    IF value->>'purpose' IS DISTINCT FROM 'imposition' OR value->'review_targets' IS DISTINCT FROM '[]'::jsonb
        OR jsonb_typeof(value->'participants') IS DISTINCT FROM 'array'
        OR octet_length(NEW.values_canonical)<7 OR get_byte(NEW.values_canonical,6)<>0 THEN
        RAISE EXCEPTION 'precautionary hearing requires imposition values and participant selections' USING ERRCODE='23514';
    END IF;
    IF jsonb_array_length(value->'participants')>32 THEN
        RAISE EXCEPTION 'precautionary hearing participant selection exceeds its bound' USING ERRCODE='23514';
    END IF;
    FOR selected IN SELECT jsonb_array_elements(value->'participants') LOOP
        selected_id:=(selected->>'id')::uuid;
        IF selected_id IS NULL OR (previous_participant IS NOT NULL AND selected_id<=previous_participant) THEN
            RAISE EXCEPTION 'precautionary hearing participant order is not canonical' USING ERRCODE='23514';
        END IF;
        previous_participant:=selected_id;
        SELECT count(*) INTO source_count FROM (
            SELECT r.revision FROM %1$I.case_participant_revisions r
            JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=NEW.case_id AND p.id=selected_id AND r.revision=(selected->>'revision')::bigint
            UNION ALL SELECT r.revision FROM %1$I.case_participant_typed_revisions r
            JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=NEW.case_id AND p.id=selected_id AND r.revision=(selected->>'revision')::bigint
        ) sources;
        IF source_count<>1 THEN
            RAISE EXCEPTION 'precautionary hearing participant must have one exact case source' USING ERRCODE='23514';
        END IF;
        IF previous.revision IS NULL OR NOT(previous.values_view->'participants' @> jsonb_build_array(selected)) THEN
            SELECT * INTO participant FROM (
                SELECT revision,directory_status FROM %1$I.case_participant_revisions WHERE participant_id=selected_id
                UNION ALL SELECT revision,directory_status FROM %1$I.case_participant_typed_revisions WHERE participant_id=selected_id
            ) revisions ORDER BY revision DESC LIMIT 1;
            IF participant.revision IS DISTINCT FROM (selected->>'revision')::bigint
                OR participant.directory_status IS DISTINCT FROM 'active' THEN
                RAISE EXCEPTION 'new precautionary hearing participant must be current and active' USING ERRCODE='23514';
            END IF;
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM %1$I.documents
        WHERE case_id=NEW.case_id AND id=(value->'scheduling_basis'->>'document_id')::uuid
            AND version=(value->'scheduling_basis'->>'version')::bigint
            AND digest=decode(value->'scheduling_basis'->>'digest','hex')) THEN
        RAISE EXCEPTION 'precautionary hearing support must match exact case content' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_precautionary_hearings','case_precautionary_hearing_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='precautionary_hearing_immutable') THEN
            EXECUTE format('CREATE TRIGGER precautionary_hearing_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_precautionary_hearing_history()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='precautionary_hearing_lock') THEN
            EXECUTE format('CREATE TRIGGER precautionary_hearing_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION lock_precautionary_hearing_history()',tab);
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_precautionary_hearing_revisions'::regclass AND tgname='precautionary_hearing_sequence') THEN
        CREATE TRIGGER precautionary_hearing_sequence BEFORE INSERT ON case_precautionary_hearing_revisions
            FOR EACH ROW EXECUTE FUNCTION enforce_precautionary_hearing_sequence();
    END IF;
END; $$;
REVOKE ALL ON FUNCTION preserve_precautionary_hearing_history(),lock_precautionary_hearing_history(),
    enforce_precautionary_hearing_sequence() FROM PUBLIC;
