DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_measure_decision_hearing_anchor() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE hearing RECORD; previous RECORD; administration RECORD; stage_source RECORD; stage_origin RECORD;
    audit_row RECORD; participant RECORD; target RECORD; target_administration RECORD; target_stage RECORD;
    candidate_administration RECORD; candidate_stage RECORD; selected JSONB; value JSONB;
    ordinal BIGINT:=0; source_count BIGINT; audit_sequence BIGINT:=-1; marker TEXT; recorded TEXT;
    admin_seconds BIGINT; admin_nanos INTEGER; source_seconds BIGINT; source_nanos INTEGER;
    prior_admin_seconds BIGINT; prior_admin_nanos INTEGER; prior_stage_seconds BIGINT; prior_stage_nanos INTEGER;
    selected_id UUID; previous_participant UUID; previous_target UUID; target_revision BIGINT;
BEGIN
    IF NEW.anchor_kind IN ('none','initial') THEN RETURN NEW; END IF;
    IF NEW.anchor_kind IS DISTINCT FROM 'precautionary'
        OR num_nonnulls(NEW.anchor_precautionary_hearing_id,NEW.anchor_revision,NEW.anchor_capture_digest)<>3
        OR num_nonnulls(NEW.anchor_hearing_id,NEW.anchor_values_digest,NEW.anchor_submission_digest)<>0
        OR NEW.anchor_revision NOT BETWEEN 1 AND 256 OR octet_length(NEW.anchor_capture_digest)<>32 THEN
        RAISE EXCEPTION 'precautionary anchor selectors exceed shape or proof bound' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.case_precautionary_hearings root
        JOIN %1$I.case_precautionary_hearing_revisions first ON first.hearing_id=root.id
            AND first.case_id=root.case_id AND first.revision=root.initial_revision
        WHERE root.id=NEW.anchor_precautionary_hearing_id AND root.case_id=NEW.case_id AND root.initial_revision=1) THEN
        RAISE EXCEPTION 'precautionary anchor root is absent' USING ERRCODE='23514';
    END IF;
    SELECT count(*) INTO source_count FROM (SELECT 1 FROM %1$I.case_precautionary_hearing_revisions
        WHERE hearing_id=NEW.anchor_precautionary_hearing_id AND case_id=NEW.case_id
            AND revision<=NEW.anchor_revision LIMIT 257) rows;
    IF source_count<>NEW.anchor_revision THEN
        RAISE EXCEPTION 'precautionary anchor prefix is incomplete' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_precautionary_hearing_revisions WHERE FALSE;
    FOR hearing IN SELECT * FROM %1$I.case_precautionary_hearing_revisions
        WHERE hearing_id=NEW.anchor_precautionary_hearing_id AND case_id=NEW.case_id
            AND revision<=NEW.anchor_revision ORDER BY revision LOOP
        ordinal:=ordinal+1;
        IF hearing.revision<>ordinal OR hearing.action NOT IN ('schedule','replace','cancel')
            OR hearing.recorded_at_seconds NOT BETWEEN -62135596800 AND 253402300799
            OR hearing.recorded_at_nanoseconds NOT BETWEEN 0 AND 999999999
            OR octet_length(hearing.submission_digest)<>32 OR octet_length(hearing.review_digest)<>32
            OR octet_length(hearing.capture_digest)<>32 OR octet_length(hearing.observed_context_digest)<>32
            OR (ordinal=1 AND (hearing.action<>'schedule' OR hearing.previous_capture_digest IS NOT NULL OR hearing.reason IS NOT NULL))
            OR (ordinal>1 AND (hearing.action='schedule' OR previous.action='cancel'
                OR hearing.previous_capture_digest IS DISTINCT FROM previous.capture_digest OR hearing.reason IS NULL
                OR ROW(hearing.recorded_at_seconds,hearing.recorded_at_nanoseconds)
                    <ROW(previous.recorded_at_seconds,previous.recorded_at_nanoseconds))) THEN
            RAISE EXCEPTION 'precautionary anchor prefix sequence or commitments differ' USING ERRCODE='23514';
        END IF;
        SELECT * INTO administration FROM %1$I.case_administration_revisions
            WHERE case_id=NEW.case_id AND revision=hearing.observed_administration_revision;
        IF administration.administrative_status IS DISTINCT FROM 'active' OR administration.nuc IS NULL
            OR administration.changed_at COLLATE "C" !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]{1,9})?Z$' THEN
            RAISE EXCEPTION 'precautionary anchor administration is not complete and active' USING ERRCODE='23514';
        END IF;
        admin_seconds:=extract(epoch FROM regexp_replace(administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
        admin_nanos:=rpad(coalesce(substring(administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
        SELECT stages.*,count(*) OVER() AS source_count INTO stage_source FROM (
            SELECT revision,administration_revision,recorded_at_seconds,recorded_at_nanoseconds
                FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id AND revision=hearing.observed_stage_revision
            UNION ALL SELECT i.stage_revision,i.administration_revision,
                extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
                rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
                FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                    ON a.case_id=i.case_id AND a.revision=i.administration_revision
                WHERE i.case_id=NEW.case_id AND i.stage_revision=hearing.observed_stage_revision
        ) stages;
        IF stage_source.source_count IS DISTINCT FROM 1::bigint
            OR stage_source.administration_revision>hearing.observed_administration_revision THEN
            RAISE EXCEPTION 'precautionary anchor stage is not exact' USING ERRCODE='23514';
        END IF;
        SELECT * INTO stage_origin FROM %1$I.case_administration_revisions
            WHERE case_id=NEW.case_id AND revision=stage_source.administration_revision;
        source_seconds:=extract(epoch FROM regexp_replace(stage_origin.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
        source_nanos:=rpad(coalesce(substring(stage_origin.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
        IF stage_origin.administrative_status IS DISTINCT FROM 'active' OR stage_origin.nuc IS NULL
            OR stage_origin.changed_at COLLATE "C" !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]{1,9})?Z$'
            OR admin_seconds NOT BETWEEN -62135596800 AND 253402300799
            OR source_seconds NOT BETWEEN -62135596800 AND 253402300799
            OR ROW(admin_seconds,admin_nanos)<ROW(source_seconds,source_nanos)
            OR ROW(stage_source.recorded_at_seconds,stage_source.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos)
            OR ROW(hearing.recorded_at_seconds,hearing.recorded_at_nanoseconds)<ROW(admin_seconds,admin_nanos)
            OR ROW(hearing.recorded_at_seconds,hearing.recorded_at_nanoseconds)
                <ROW(stage_source.recorded_at_seconds,stage_source.recorded_at_nanoseconds)
            OR (ordinal>1 AND (hearing.observed_administration_revision<previous.observed_administration_revision
                OR hearing.observed_stage_revision<previous.observed_stage_revision
                OR ROW(admin_seconds,admin_nanos)<ROW(prior_admin_seconds,prior_admin_nanos)
                OR ROW(stage_source.recorded_at_seconds,stage_source.recorded_at_nanoseconds)
                    <ROW(prior_stage_seconds,prior_stage_nanos))) THEN
            RAISE EXCEPTION 'precautionary anchor context chronology differs' USING ERRCODE='23514';
        END IF;
        marker:='ph1:case:'||NEW.case_id::text||':hearing:'||hearing.hearing_id::text
            ||':operation:'||hearing.operation_id::text||':revision:'||hearing.revision::text
            ||':submission:'||encode(hearing.submission_digest,'hex')||':review:'||encode(hearing.review_digest,'hex')
            ||':capture:'||encode(hearing.capture_digest,'hex');
        SELECT count(*) INTO source_count FROM (SELECT 1 FROM %1$I.audit_events e
            WHERE e.resource COLLATE "C"=marker COLLATE "C" AND e.action IN
                ('precautionary_hearing.schedule','precautionary_hearing.replace','precautionary_hearing.cancel') LIMIT 2) events;
        SELECT e.sequence,e.actor,e.action,e.timestamp,e.resource INTO audit_row FROM %1$I.audit_events e
            WHERE e.sequence=hearing.audit_sequence AND octet_length(e.timestamp) BETWEEN 1 AND 64
                AND octet_length(e.actor) BETWEEN 1 AND 1280 AND octet_length(e.action) BETWEEN 1 AND 64
                AND octet_length(e.resource) BETWEEN 1 AND 512 AND octet_length(e.chain)=32;
        recorded:=to_char(to_timestamp(hearing.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
            ||CASE WHEN hearing.recorded_at_nanoseconds=0 THEN 'Z'
                ELSE '.'||rtrim(lpad(hearing.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END;
        IF source_count<>1 OR audit_row.sequence IS NULL OR audit_row.sequence<=audit_sequence
            OR audit_row.actor COLLATE "C" IS DISTINCT FROM hearing.recorded_by_email COLLATE "C"
            OR audit_row.action COLLATE "C" IS DISTINCT FROM ('precautionary_hearing.'||hearing.action) COLLATE "C"
            OR audit_row.resource COLLATE "C" IS DISTINCT FROM marker COLLATE "C"
            OR audit_row.timestamp COLLATE "C" IS DISTINCT FROM recorded COLLATE "C" THEN
            RAISE EXCEPTION 'precautionary anchor original audit differs from its receipt' USING ERRCODE='23514';
        END IF;
        audit_sequence:=audit_row.sequence;
        IF hearing.action='cancel' THEN
            IF num_nonnulls(hearing.values_canonical,hearing.values_view,hearing.values_digest,
                hearing.support_format,hearing.support_policy)<>0 THEN
                RAISE EXCEPTION 'precautionary cancellation changed retained selections' USING ERRCODE='23514';
            END IF;
        ELSE
            value:=hearing.values_view;
            IF num_nonnulls(hearing.values_canonical,hearing.values_view,hearing.values_digest,
                    hearing.support_format,hearing.support_policy)<>5
                OR octet_length(hearing.values_canonical) NOT BETWEEN 7 AND 16395
                OR substring(hearing.values_canonical FROM 1 FOR 6)<>convert_to('PHEAR1','UTF8')
                OR hearing.values_digest IS DISTINCT FROM sha256(hearing.values_canonical)
                OR jsonb_typeof(value) IS DISTINCT FROM 'object' OR octet_length(value::text)>65536
                OR jsonb_typeof(value->'participants') IS DISTINCT FROM 'array'
                OR jsonb_typeof(value->'review_targets') IS DISTINCT FROM 'array'
                OR hearing.support_format NOT IN ('pdf','docx') OR hearing.support_policy<>'pdf_docx_v1' THEN
                RAISE EXCEPTION 'precautionary anchor values shape or commitment differs' USING ERRCODE='23514';
            END IF;
            IF NOT(value ?& ARRAY['purpose','time','modality','venue','note','participants','scheduling_basis','review_targets'])
                OR value-ARRAY['purpose','time','modality','venue','note','participants','scheduling_basis','review_targets']<>'{}'::jsonb
                OR jsonb_typeof(value->'scheduling_basis') IS DISTINCT FROM 'object'
                OR jsonb_array_length(value->'participants')>32 OR jsonb_array_length(value->'review_targets')>32 THEN
                RAISE EXCEPTION 'precautionary anchor selection exceeds bound' USING ERRCODE='23514';
            END IF;
            previous_participant:=NULL;
            FOR selected IN SELECT jsonb_array_elements(value->'participants') LOOP
                IF jsonb_typeof(selected) IS DISTINCT FROM 'object' THEN
                    RAISE EXCEPTION 'precautionary participant must be an object' USING ERRCODE='23514';
                END IF;
                IF NOT(selected ?& ARRAY['id','revision']) OR selected-ARRAY['id','revision']<>'{}'::jsonb
                    OR jsonb_typeof(selected->'id') IS DISTINCT FROM 'string'
                    OR coalesce(selected->>'id','') COLLATE "C" !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                    OR jsonb_typeof(selected->'revision') IS DISTINCT FROM 'number'
                    OR coalesce(selected->>'revision','') COLLATE "C" !~ '^[1-9][0-9]{0,9}$' THEN
                    RAISE EXCEPTION 'precautionary participant selector is not canonical' USING ERRCODE='23514';
                END IF;
                selected_id:=(selected->>'id')::uuid;
                IF (selected->>'revision')::bigint>4294967295
                    OR (previous_participant IS NOT NULL AND selected_id<=previous_participant) THEN
                    RAISE EXCEPTION 'precautionary participant revision or order differs' USING ERRCODE='23514';
                END IF;
                previous_participant:=selected_id;
                SELECT sources.*,count(*) OVER() AS source_count INTO participant FROM (
                    SELECT r.directory_status,r.changed_at,NULL::text AS subject_changed_at FROM %1$I.case_participant_revisions r
                        JOIN %1$I.case_participants p ON p.id=r.participant_id
                        WHERE p.case_id=NEW.case_id AND p.id=selected_id AND r.revision=(selected->>'revision')::bigint
                    UNION ALL SELECT r.directory_status,r.changed_at,s.changed_at FROM %1$I.case_participant_typed_revisions r
                        JOIN %1$I.case_participants p ON p.id=r.participant_id
                        JOIN %1$I.case_subject_revisions s ON s.subject_id=r.subject_id AND s.revision=r.subject_revision
                        JOIN %1$I.case_subjects root ON root.id=s.subject_id AND root.case_id=p.case_id
                        WHERE p.case_id=NEW.case_id AND p.id=selected_id AND r.revision=(selected->>'revision')::bigint
                ) sources;
                IF participant.source_count IS DISTINCT FROM 1::bigint
                    OR participant.changed_at COLLATE "C" !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]{1,9})?Z$'
                    OR ((ordinal=1 OR NOT(previous.values_view->'participants' @> jsonb_build_array(selected)))
                        AND participant.directory_status IS DISTINCT FROM 'active') THEN
                    RAISE EXCEPTION 'precautionary participant historical source differs' USING ERRCODE='23514';
                END IF;
                source_seconds:=extract(epoch FROM regexp_replace(participant.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
                source_nanos:=rpad(coalesce(substring(participant.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
                IF source_seconds NOT BETWEEN -62135596800 AND 253402300799
                    OR ROW(hearing.recorded_at_seconds,hearing.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
                    RAISE EXCEPTION 'precautionary anchor predates its participant' USING ERRCODE='23514';
                END IF;
                IF participant.subject_changed_at IS NOT NULL THEN
                    source_seconds:=extract(epoch FROM regexp_replace(participant.subject_changed_at,'\.[0-9]+','')::timestamptz)::bigint;
                    source_nanos:=rpad(coalesce(substring(participant.subject_changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
                    IF participant.subject_changed_at COLLATE "C" !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]{1,9})?Z$'
                        OR source_seconds NOT BETWEEN -62135596800 AND 253402300799
                        OR ROW(hearing.recorded_at_seconds,hearing.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
                        RAISE EXCEPTION 'precautionary anchor predates its subject' USING ERRCODE='23514';
                    END IF;
                END IF;
            END LOOP;
            IF NOT EXISTS(SELECT 1 FROM %1$I.documents
                WHERE case_id=NEW.case_id AND id=(value->'scheduling_basis'->>'document_id')::uuid
                    AND version=(value->'scheduling_basis'->>'version')::bigint
                    AND digest=decode(value->'scheduling_basis'->>'digest','hex')) THEN
                RAISE EXCEPTION 'precautionary anchor support differs from exact case content' USING ERRCODE='23514';
            END IF;
            IF value->>'purpose'='imposition' THEN
                IF jsonb_array_length(value->'review_targets')<>0 OR get_byte(hearing.values_canonical,6)<>0 THEN
                    RAISE EXCEPTION 'imposition anchor cannot select review targets' USING ERRCODE='23514';
                END IF;
            ELSIF value->>'purpose'='review' THEN
                IF jsonb_array_length(value->'review_targets')=0 OR get_byte(hearing.values_canonical,6)<>1 THEN
                    RAISE EXCEPTION 'review anchor requires exact targets' USING ERRCODE='23514';
                END IF;
                previous_target:=NULL;
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
                    selected_id:=(selected->>'id')::uuid; target_revision:=(selected->>'revision')::bigint;
                    IF target_revision>4294967295 OR (previous_target IS NOT NULL AND selected_id<=previous_target) THEN
                        RAISE EXCEPTION 'review target revision or order differs' USING ERRCODE='23514';
                    END IF;
                    previous_target:=selected_id;
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
                        WHERE r.measure_id=selected_id AND r.revision=target_revision AND r.case_id=NEW.case_id
                            AND r.family='m1' AND r.capture_digest=decode(selected->>'digest','hex');
                    IF target.case_id IS DISTINCT FROM NEW.case_id
                        OR target.action NOT IN ('impose','confirm','modify','revoke','cease','substitute_out','substitute_in')
                        OR target.observed_administration_revision>hearing.observed_administration_revision
                        OR target.observed_stage_revision>hearing.observed_stage_revision
                        OR ROW(hearing.recorded_at_seconds,hearing.recorded_at_nanoseconds)<ROW(target.recorded_at_seconds,target.recorded_at_nanoseconds) THEN
                        RAISE EXCEPTION 'review target must have exact case ownership and precede scheduling' USING ERRCODE='23514';
                    END IF;
                    SELECT * INTO target_administration FROM %1$I.case_administration_revisions
                        WHERE case_id=NEW.case_id AND revision=target.observed_administration_revision;
                    IF target_administration.administrative_status IS DISTINCT FROM 'active' OR target_administration.nuc IS NULL THEN
                        RAISE EXCEPTION 'review target administration is not complete and active' USING ERRCODE='23514';
                    END IF;
                    source_seconds:=extract(epoch FROM regexp_replace(target_administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
                    source_nanos:=rpad(coalesce(substring(target_administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
                    IF ROW(admin_seconds,admin_nanos)<ROW(source_seconds,source_nanos) THEN
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
                        OR ROW(stage_source.recorded_at_seconds,stage_source.recorded_at_nanoseconds)
                            <ROW(target_stage.recorded_at_seconds,target_stage.recorded_at_nanoseconds) THEN
                        RAISE EXCEPTION 'review scheduling stage predates exact target context' USING ERRCODE='23514';
                    END IF;
                END LOOP;
            ELSE
                RAISE EXCEPTION 'unsupported precautionary anchor purpose' USING ERRCODE='23514';
            END IF;
        END IF;
        prior_admin_seconds:=admin_seconds; prior_admin_nanos:=admin_nanos;
        prior_stage_seconds:=stage_source.recorded_at_seconds; prior_stage_nanos:=stage_source.recorded_at_nanoseconds;
        previous:=hearing;
    END LOOP;
    SELECT * INTO candidate_administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id AND revision=NEW.observed_administration_revision;
    source_seconds:=extract(epoch FROM regexp_replace(candidate_administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
    source_nanos:=rpad(coalesce(substring(candidate_administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    SELECT stages.*,count(*) OVER() AS source_count INTO candidate_stage FROM (
        SELECT revision,recorded_at_seconds,recorded_at_nanoseconds FROM %1$I.case_stage_revisions
            WHERE case_id=NEW.case_id AND revision=NEW.observed_stage_revision
        UNION ALL SELECT i.stage_revision,
            extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
            rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
            FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                ON a.case_id=i.case_id AND a.revision=i.administration_revision
            WHERE i.case_id=NEW.case_id AND i.stage_revision=NEW.observed_stage_revision
    ) stages;
    IF ordinal<>NEW.anchor_revision OR previous.capture_digest IS DISTINCT FROM NEW.anchor_capture_digest
        OR candidate_administration.revision IS NULL OR candidate_stage.source_count IS DISTINCT FROM 1::bigint
        OR previous.observed_administration_revision>NEW.observed_administration_revision
        OR previous.observed_stage_revision>NEW.observed_stage_revision
        OR ROW(source_seconds,source_nanos)<ROW(prior_admin_seconds,prior_admin_nanos)
        OR ROW(candidate_stage.recorded_at_seconds,candidate_stage.recorded_at_nanoseconds)<ROW(prior_stage_seconds,prior_stage_nanos)
        OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)
            <ROW(previous.recorded_at_seconds,previous.recorded_at_nanoseconds) THEN
        RAISE EXCEPTION 'precautionary anchor selection context or capture time differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION enforce_measure_decision_hearing_anchor() FROM PUBLIC;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_measure_decisions'::regclass
        AND tgname='measure_decision_hearing_anchor' AND NOT tgisinternal) THEN
        CREATE TRIGGER measure_decision_hearing_anchor BEFORE INSERT ON case_measure_decisions
            FOR EACH ROW EXECUTE FUNCTION enforce_measure_decision_hearing_anchor();
    END IF;
END; $$;
