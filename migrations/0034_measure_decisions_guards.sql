CREATE OR REPLACE FUNCTION preserve_measure_decision_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'measure decision history is immutable' USING ERRCODE='23514';
END; $$;
CREATE OR REPLACE FUNCTION lock_measure_decision_history() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'measure decision writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_measure_decision_capture() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE administration RECORD; actor RECORD; current_stage RECORD; operation RECORD;
    source_count BIGINT; source_seconds BIGINT; source_nanos INTEGER; stage_origin_time TEXT;
    audit RECORD; recorded TEXT; marker TEXT; value JSONB; effect JSONB;
    previous_id UUID; selected_id UUID;
BEGIN
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE
        OR actor.email COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C"
        OR actor.role COLLATE "C" IS DISTINCT FROM NEW.recorded_by_role COLLATE "C"
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by FOR SHARE))) THEN
        RAISE EXCEPTION 'measure decision actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS DISTINCT FROM NEW.observed_administration_revision
        OR administration.administrative_status IS DISTINCT FROM 'active' OR administration.nuc IS NULL THEN
        RAISE EXCEPTION 'measure decision requires current active complete administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO current_stage FROM (
        SELECT revision,administration_revision,recorded_at_seconds,recorded_at_nanoseconds
            FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id
        UNION ALL SELECT stage_revision,administration_revision,NULL::bigint,NULL::integer
            FROM %1$I.case_initial_stage_registrations WHERE case_id=NEW.case_id
    ) stages ORDER BY revision DESC LIMIT 1;
    SELECT count(*) INTO source_count FROM (
        SELECT revision FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id AND revision=NEW.observed_stage_revision
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
        RAISE EXCEPTION 'measure decision requires the exact current stage' USING ERRCODE='23514';
    END IF;
    source_seconds:=extract(epoch FROM regexp_replace(administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
    source_nanos:=rpad(coalesce(substring(administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    IF ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
        RAISE EXCEPTION 'measure decision predates observed administration' USING ERRCODE='23514';
    END IF;
    SELECT changed_at INTO stage_origin_time FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id AND revision=current_stage.administration_revision;
    source_seconds:=extract(epoch FROM regexp_replace(stage_origin_time,'\.[0-9]+','')::timestamptz)::bigint;
    source_nanos:=rpad(coalesce(substring(stage_origin_time FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    IF stage_origin_time IS NULL
        OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
        RAISE EXCEPTION 'measure decision predates stage origin' USING ERRCODE='23514';
    END IF;
    IF jsonb_typeof(NEW.values_view) IS DISTINCT FROM 'object' OR octet_length(NEW.values_view::text)>32768
        OR jsonb_typeof(NEW.outcome_view) IS DISTINCT FROM 'object' OR octet_length(NEW.outcome_view::text)>1048576 THEN
        RAISE EXCEPTION 'measure decision projection exceeds its shape or bound' USING ERRCODE='23514';
    END IF;
    value:=NEW.values_view;
    IF NOT(value ?& ARRAY['authority','declared_at','justification','support','locator'])
        OR value-ARRAY['authority','declared_at','justification','support','locator']<>'{}'::jsonb
        OR jsonb_typeof(value->'support') IS DISTINCT FROM 'object'
        OR NOT((value->'support') ?& ARRAY['document_id','version','digest'])
        OR (value->'support')-ARRAY['document_id','version','digest']<>'{}'::jsonb
        OR NOT EXISTS(SELECT 1 FROM %1$I.documents
            WHERE case_id=NEW.case_id AND id=(value->'support'->>'document_id')::uuid
                AND version=(value->'support'->>'version')::bigint
                AND digest=decode(value->'support'->>'digest','hex')) THEN
        RAISE EXCEPTION 'measure decision support must match exact case content' USING ERRCODE='23514';
    END IF;
    value:=NEW.outcome_view;
    IF value->>'kind'='no_measure_change' THEN
        IF NOT(value ?& ARRAY['kind','statement']) OR value-ARRAY['kind','statement']<>'{}'::jsonb
            OR jsonb_typeof(value->'statement') IS DISTINCT FROM 'string'
            OR %1$I.case_administration_text_valid(value->>'statement',1000,TRUE) IS DISTINCT FROM TRUE
            OR get_byte(NEW.outcome_canonical,5)<>1 THEN
            RAISE EXCEPTION 'invalid no-measure-change outcome' USING ERRCODE='23514';
        END IF;
    ELSIF value->>'kind'='changes' THEN
        IF NOT(value ?& ARRAY['kind','effects']) OR value-ARRAY['kind','effects']<>'{}'::jsonb
            OR jsonb_typeof(value->'effects') IS DISTINCT FROM 'array' THEN
            RAISE EXCEPTION 'invalid measure change outcome' USING ERRCODE='23514';
        END IF;
        IF jsonb_array_length(value->'effects') NOT BETWEEN 1 AND 32 OR get_byte(NEW.outcome_canonical,5)<>0 THEN
            RAISE EXCEPTION 'measure decision effect count exceeds bound' USING ERRCODE='23514';
        END IF;
        FOR effect IN SELECT jsonb_array_elements(value->'effects') LOOP
            IF jsonb_typeof(effect) IS DISTINCT FROM 'object' OR effect->>'action' IS DISTINCT FROM 'impose'
                OR NOT(effect ?& ARRAY['action','proposal']) OR effect-ARRAY['action','proposal']<>'{}'::jsonb
                OR jsonb_typeof(effect->'proposal') IS DISTINCT FROM 'object'
                OR NOT((effect->'proposal') ?& ARRAY['id','values'])
                OR (effect->'proposal')-ARRAY['id','values']<>'{}'::jsonb THEN
                RAISE EXCEPTION 'only standalone imposition is persisted' USING ERRCODE='23514';
            END IF;
            selected_id:=(effect->'proposal'->>'id')::uuid;
            IF selected_id IS NULL OR (previous_id IS NOT NULL AND selected_id<=previous_id) THEN
                RAISE EXCEPTION 'measure proposal identity order differs' USING ERRCODE='23514';
            END IF;
            previous_id:=selected_id;
        END LOOP;
    ELSE
        RAISE EXCEPTION 'unsupported measure decision outcome' USING ERRCODE='23514';
    END IF;
    SELECT * INTO operation FROM %1$I.case_measure_operations WHERE operation_id=NEW.operation_id;
    IF operation.case_id IS DISTINCT FROM NEW.case_id OR operation.family IS DISTINCT FROM 'g1'
        OR operation.owner_digest IS DISTINCT FROM NEW.group_digest THEN
        RAISE EXCEPTION 'measure decision owner differs' USING ERRCODE='23514';
    END IF;
    recorded:=to_char(to_timestamp(NEW.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
        ||CASE WHEN NEW.recorded_at_nanoseconds=0 THEN 'Z'
            ELSE '.'||rtrim(lpad(NEW.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END;
    marker:='mg1:case:'||NEW.case_id::text||':operation:'||NEW.operation_id::text||':decision:'||NEW.decision_id::text
        ||':submission:'||encode(NEW.submission_digest,'hex')||':review:'||encode(NEW.review_digest,'hex')
        ||':decision_digest:'||encode(NEW.decision_digest,'hex')||':group:'||encode(NEW.group_digest,'hex');
    SELECT * INTO audit FROM %1$I.audit_events WHERE sequence=operation.audit_sequence;
    IF audit.actor COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C"
        OR audit.action COLLATE "C" IS DISTINCT FROM 'measure_decision.recorded' COLLATE "C"
        OR audit.resource COLLATE "C" IS DISTINCT FROM marker COLLATE "C"
        OR audit.timestamp COLLATE "C" IS DISTINCT FROM recorded COLLATE "C" THEN
        RAISE EXCEPTION 'measure decision audit marker differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION preserve_measure_decision_history(),lock_measure_decision_history(),
    enforce_measure_decision_capture() FROM PUBLIC;
