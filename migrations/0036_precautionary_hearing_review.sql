DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_precautionary_hearing_sequence() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; administration RECORD; actor RECORD; current_stage RECORD;
    value JSONB; selected JSONB; participant RECORD; source_count BIGINT;
    previous_participant UUID; selected_id UUID; source_seconds BIGINT; source_nanos INTEGER;
    audit RECORD; recorded TEXT; marker TEXT; canonical BYTEA;
    scheduling_administration RECORD; scheduling_stage RECORD; target RECORD; target_administration RECORD; target_stage RECORD;
    scheduling_admin_revision BIGINT; scheduling_stage_revision BIGINT; target_revision BIGINT;
    scheduling_seconds BIGINT; scheduling_nanos INTEGER; previous_target UUID; target_id UUID;
    prior_administration RECORD; prior_stage RECORD; prior_seconds BIGINT; prior_nanos INTEGER;
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
        UNION ALL SELECT i.stage_revision,i.administration_revision,
            extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
            rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
            FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                ON a.case_id=i.case_id AND a.revision=i.administration_revision
            WHERE i.case_id=NEW.case_id
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
    IF previous.revision IS NOT NULL THEN
        SELECT * INTO prior_administration FROM %1$I.case_administration_revisions
            WHERE case_id=NEW.case_id AND revision=previous.observed_administration_revision;
        prior_seconds:=extract(epoch FROM regexp_replace(prior_administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
        prior_nanos:=rpad(coalesce(substring(prior_administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
        IF prior_administration.revision IS NULL
            OR NEW.observed_administration_revision<previous.observed_administration_revision
            OR ROW(source_seconds,source_nanos)<ROW(prior_seconds,prior_nanos) THEN
            RAISE EXCEPTION 'precautionary hearing observed administration regressed' USING ERRCODE='23514';
        END IF;
        SELECT stages.*,count(*) OVER() AS source_count INTO prior_stage FROM (
            SELECT revision,recorded_at_seconds,recorded_at_nanoseconds
                FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id AND revision=previous.observed_stage_revision
            UNION ALL SELECT i.stage_revision,
                extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
                rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
                FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                    ON a.case_id=i.case_id AND a.revision=i.administration_revision
                WHERE i.case_id=NEW.case_id AND i.stage_revision=previous.observed_stage_revision
        ) stages;
        IF prior_stage.source_count IS DISTINCT FROM 1::bigint
            OR NEW.observed_stage_revision<previous.observed_stage_revision
            OR ROW(current_stage.recorded_at_seconds,current_stage.recorded_at_nanoseconds)
                <ROW(prior_stage.recorded_at_seconds,prior_stage.recorded_at_nanoseconds) THEN
            RAISE EXCEPTION 'precautionary hearing observed stage regressed' USING ERRCODE='23514';
        END IF;
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
    IF NEW.action='cancel' THEN
        value:=previous.values_view; canonical:=previous.values_canonical;
        scheduling_admin_revision:=previous.observed_administration_revision;
        scheduling_stage_revision:=previous.observed_stage_revision;
    ELSE
        value:=NEW.values_view; canonical:=NEW.values_canonical;
        scheduling_admin_revision:=NEW.observed_administration_revision;
        scheduling_stage_revision:=NEW.observed_stage_revision;
    END IF;
    IF jsonb_typeof(value) IS DISTINCT FROM 'object'
        OR jsonb_typeof(value->'participants') IS DISTINCT FROM 'array'
        OR jsonb_typeof(value->'review_targets') IS DISTINCT FROM 'array'
        OR canonical IS NULL OR octet_length(canonical)<7 THEN
        RAISE EXCEPTION 'precautionary hearing values or selections are malformed' USING ERRCODE='23514';
    END IF;
    IF jsonb_array_length(value->'participants')>32 OR jsonb_array_length(value->'review_targets')>32 THEN
        RAISE EXCEPTION 'precautionary hearing selection exceeds its bound' USING ERRCODE='23514';
    END IF;
    IF value->>'purpose'='imposition' THEN
        IF jsonb_array_length(value->'review_targets')<>0 OR get_byte(canonical,6)<>0 THEN
            RAISE EXCEPTION 'imposition hearing cannot select review targets' USING ERRCODE='23514';
        END IF;
    ELSIF value->>'purpose'='review' THEN
        IF jsonb_array_length(value->'review_targets')=0 OR get_byte(canonical,6)<>1 THEN
            RAISE EXCEPTION 'review hearing requires exact targets' USING ERRCODE='23514';
        END IF;
        SELECT * INTO scheduling_administration FROM %1$I.case_administration_revisions
            WHERE case_id=NEW.case_id AND revision=scheduling_admin_revision;
        IF scheduling_administration.administrative_status IS DISTINCT FROM 'active'
            OR scheduling_administration.nuc IS NULL THEN
            RAISE EXCEPTION 'review scheduling administration is not complete and active' USING ERRCODE='23514';
        END IF;
        scheduling_seconds:=extract(epoch FROM regexp_replace(scheduling_administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
        scheduling_nanos:=rpad(coalesce(substring(scheduling_administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
        SELECT stages.*,count(*) OVER() AS source_count INTO scheduling_stage FROM (
            SELECT revision,administration_revision,recorded_at_seconds,recorded_at_nanoseconds
                FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id AND revision=scheduling_stage_revision
            UNION ALL SELECT i.stage_revision,i.administration_revision,
                extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
                rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
                FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                    ON a.case_id=i.case_id AND a.revision=i.administration_revision
                WHERE i.case_id=NEW.case_id AND i.stage_revision=scheduling_stage_revision
        ) stages;
        IF scheduling_stage.source_count IS DISTINCT FROM 1::bigint
            OR scheduling_stage.administration_revision>scheduling_admin_revision THEN
            RAISE EXCEPTION 'review scheduling stage is not exact' USING ERRCODE='23514';
        END IF;
        FOR selected IN SELECT jsonb_array_elements(value->'review_targets') LOOP
            IF jsonb_typeof(selected) IS DISTINCT FROM 'object' THEN
                RAISE EXCEPTION 'review target must be an object' USING ERRCODE='23514';
            END IF;
            IF NOT(selected ?& ARRAY['id','revision','digest']) OR selected-ARRAY['id','revision','digest']<>'{}'::jsonb
                OR jsonb_typeof(selected->'id') IS DISTINCT FROM 'string'
                OR coalesce(selected->>'id','') COLLATE "C" !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                OR jsonb_typeof(selected->'revision') IS DISTINCT FROM 'number'
                OR coalesce(selected->>'revision','') COLLATE "C" !~ '^[1-9][0-9]{0,9}$'
                OR jsonb_typeof(selected->'digest') IS DISTINCT FROM 'string'
                OR coalesce(selected->>'digest','') COLLATE "C" !~ '^[0-9a-f]{64}$' THEN
                RAISE EXCEPTION 'review target fields are not canonical' USING ERRCODE='23514';
            END IF;
            target_id:=(selected->>'id')::uuid; target_revision:=(selected->>'revision')::bigint;
            IF target_revision>4294967295 OR (previous_target IS NOT NULL AND target_id<=previous_target) THEN
                RAISE EXCEPTION 'review target revision or order differs' USING ERRCODE='23514';
            END IF;
            previous_target:=target_id;
            SELECT r.*,d.observed_administration_revision,d.observed_stage_revision,
                d.recorded_at_seconds,d.recorded_at_nanoseconds INTO target
                FROM %1$I.case_measure_revisions r JOIN %1$I.case_measure_operations o
                    ON o.operation_id=r.owner_operation AND o.case_id=r.case_id AND o.family='g1'
                JOIN %1$I.case_measure_decisions d ON d.operation_id=o.operation_id AND d.case_id=o.case_id
                    AND d.group_digest=o.owner_digest
                JOIN %1$I.case_measures root ON root.id=r.measure_id AND root.case_id=r.case_id AND root.initial_revision=1
                JOIN %1$I.case_measure_revisions first ON first.measure_id=root.id AND first.case_id=root.case_id
                    AND first.revision=1 AND first.owner_operation=root.root_operation AND first.family='m1'
                    AND first.action IN ('impose','substitute_in')
                WHERE r.measure_id=target_id AND r.revision=target_revision AND r.case_id=NEW.case_id
                    AND r.family='m1' AND r.capture_digest=decode(selected->>'digest','hex');
            IF target.case_id IS DISTINCT FROM NEW.case_id
                OR target.action NOT IN ('impose','confirm','modify','revoke','cease','substitute_out','substitute_in')
                OR target.observed_administration_revision>scheduling_admin_revision
                OR target.observed_stage_revision>scheduling_stage_revision
                OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(target.recorded_at_seconds,target.recorded_at_nanoseconds)
                OR (NEW.action='cancel' AND ROW(previous.recorded_at_seconds,previous.recorded_at_nanoseconds)
                    <ROW(target.recorded_at_seconds,target.recorded_at_nanoseconds)) THEN
                RAISE EXCEPTION 'review target must have exact case ownership and precede scheduling' USING ERRCODE='23514';
            END IF;
            SELECT * INTO target_administration FROM %1$I.case_administration_revisions
                WHERE case_id=NEW.case_id AND revision=target.observed_administration_revision;
            IF target_administration.administrative_status IS DISTINCT FROM 'active' OR target_administration.nuc IS NULL THEN
                RAISE EXCEPTION 'review target administration is not complete and active' USING ERRCODE='23514';
            END IF;
            source_seconds:=extract(epoch FROM regexp_replace(target_administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
            source_nanos:=rpad(coalesce(substring(target_administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
            IF ROW(scheduling_seconds,scheduling_nanos)<ROW(source_seconds,source_nanos) THEN
                RAISE EXCEPTION 'review scheduling administration predates target context' USING ERRCODE='23514';
            END IF;
            SELECT stages.*,count(*) OVER() AS source_count INTO target_stage FROM (
                SELECT revision,administration_revision,recorded_at_seconds,recorded_at_nanoseconds
                    FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id AND revision=target.observed_stage_revision
                UNION ALL SELECT i.stage_revision,i.administration_revision,
                    extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
                    rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
                    FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                        ON a.case_id=i.case_id AND a.revision=i.administration_revision
                    WHERE i.case_id=NEW.case_id AND i.stage_revision=target.observed_stage_revision
            ) stages;
            IF target_stage.source_count IS DISTINCT FROM 1::bigint
                OR target_stage.administration_revision>target.observed_administration_revision
                OR ROW(scheduling_stage.recorded_at_seconds,scheduling_stage.recorded_at_nanoseconds)
                    <ROW(target_stage.recorded_at_seconds,target_stage.recorded_at_nanoseconds) THEN
                RAISE EXCEPTION 'review scheduling stage predates exact target context' USING ERRCODE='23514';
            END IF;
        END LOOP;
    ELSE
        RAISE EXCEPTION 'unsupported precautionary hearing purpose' USING ERRCODE='23514';
    END IF;
    IF NEW.action='cancel' THEN RETURN NEW; END IF;
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
REVOKE ALL ON FUNCTION enforce_precautionary_hearing_sequence() FROM PUBLIC;
