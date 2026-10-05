DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_measure_decision_capture() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE administration RECORD; actor RECORD; current_stage RECORD; operation RECORD;
    source_count BIGINT; source_seconds BIGINT; source_nanos INTEGER; stage_origin_time TEXT;
    audit RECORD; recorded TEXT; marker TEXT; value JSONB; effect JSONB;
    previous_id UUID; selected_id UUID; effect_key UUID; old_key UUID; subject_key UUID;
    effect_action TEXT; selected JSONB; side JSONB; side_index INTEGER; total INTEGER:=0;
    all_ids UUID[]:=ARRAY[]::uuid[]; prior RECORD; old_revision BIGINT;
    anchor_row RECORD; anchor_previous RECORD; anchor_stage RECORD; anchor_audit RECORD;
    anchor_ordinal BIGINT:=0; anchor_audit_sequence BIGINT:=-1; anchor_marker TEXT; anchor_time TEXT;
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
    IF NEW.anchor_kind='none' THEN
        IF num_nonnulls(NEW.anchor_hearing_id,NEW.anchor_revision,NEW.anchor_values_digest,NEW.anchor_submission_digest)<>0 THEN
            RAISE EXCEPTION 'unanchored decision contains hearing selectors' USING ERRCODE='23514';
        END IF;
    ELSIF NEW.anchor_kind='initial' THEN
        IF num_nonnulls(NEW.anchor_hearing_id,NEW.anchor_revision,NEW.anchor_values_digest,NEW.anchor_submission_digest)<>4
            OR NEW.anchor_revision NOT BETWEEN 1 AND 256
            OR octet_length(NEW.anchor_values_digest)<>32 OR octet_length(NEW.anchor_submission_digest)<>32 THEN
            RAISE EXCEPTION 'initial anchor selectors exceed their shape or proof bound' USING ERRCODE='23514';
        END IF;
        IF NOT EXISTS(SELECT 1 FROM %1$I.case_hearings root JOIN %1$I.case_hearing_revisions first
            ON first.hearing_id=root.id AND first.case_id=root.case_id AND first.revision=root.initial_revision
            WHERE root.id=NEW.anchor_hearing_id AND root.case_id=NEW.case_id AND root.initial_revision=1) THEN
            RAISE EXCEPTION 'initial anchor root is absent' USING ERRCODE='23514';
        END IF;
        SELECT count(*) INTO source_count FROM (SELECT 1 FROM %1$I.case_hearing_revisions
            WHERE hearing_id=NEW.anchor_hearing_id AND case_id=NEW.case_id AND revision<=NEW.anchor_revision LIMIT 257) rows;
        IF source_count<>NEW.anchor_revision THEN
            RAISE EXCEPTION 'initial anchor prefix is incomplete' USING ERRCODE='23514';
        END IF;
        FOR anchor_row IN SELECT * FROM %1$I.case_hearing_revisions
            WHERE hearing_id=NEW.anchor_hearing_id AND case_id=NEW.case_id AND revision<=NEW.anchor_revision ORDER BY revision LOOP
            anchor_ordinal:=anchor_ordinal+1;
            IF anchor_row.revision<>anchor_ordinal OR anchor_row.values_view->>'kind' IS DISTINCT FROM 'initial'
                OR octet_length(anchor_row.values_canonical) NOT BETWEEN 27 AND 10726
                OR octet_length(anchor_row.submission_canonical) NOT BETWEEN 108 AND 4120
                OR anchor_row.values_digest IS DISTINCT FROM sha256(anchor_row.values_canonical)
                OR anchor_row.submission_digest IS DISTINCT FROM sha256(anchor_row.submission_canonical)
                OR (anchor_ordinal=1 AND anchor_row.action<>'schedule')
                OR (anchor_ordinal>1 AND anchor_row.action NOT IN ('replace','cancel')) THEN
                RAISE EXCEPTION 'initial anchor prefix kind sequence or commitments differ' USING ERRCODE='23514';
            END IF;
            IF anchor_ordinal>1 THEN
                IF anchor_previous.action='cancel' THEN
                    RAISE EXCEPTION 'initial anchor follows a cancelled predecessor' USING ERRCODE='23514';
                END IF;
                IF anchor_row.action='cancel' AND ROW(anchor_row.values_canonical,anchor_row.values_digest,
                    anchor_row.scheduling_administration_revision,anchor_row.scheduling_administration_digest,
                    anchor_row.scheduling_stage_revision,anchor_row.scheduling_stage,anchor_row.scheduling_stage_digest,
                    anchor_row.support_name,anchor_row.support_format,anchor_row.support_policy)
                    IS DISTINCT FROM ROW(anchor_previous.values_canonical,anchor_previous.values_digest,
                    anchor_previous.scheduling_administration_revision,anchor_previous.scheduling_administration_digest,
                    anchor_previous.scheduling_stage_revision,anchor_previous.scheduling_stage,anchor_previous.scheduling_stage_digest,
                    anchor_previous.support_name,anchor_previous.support_format,anchor_previous.support_policy) THEN
                    RAISE EXCEPTION 'initial cancellation changed retained scheduling sources' USING ERRCODE='23514';
                END IF;
            END IF;
            IF NOT EXISTS(SELECT 1 FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id
                    AND revision=anchor_row.scheduling_administration_revision AND values_digest=anchor_row.scheduling_administration_digest)
                OR NOT EXISTS(SELECT 1 FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id
                    AND revision=anchor_row.recorded_administration_revision AND values_digest=anchor_row.recorded_administration_digest) THEN
                RAISE EXCEPTION 'initial anchor administrative sources differ' USING ERRCODE='23514';
            END IF;
            SELECT stages.*,count(*) OVER() AS source_count INTO anchor_stage FROM (
                SELECT revision,stage,values_digest FROM %1$I.case_stage_revisions
                    WHERE case_id=NEW.case_id AND revision=anchor_row.scheduling_stage_revision
                UNION ALL SELECT stage_revision,stage,NULL::bytea FROM %1$I.case_initial_stage_registrations
                    WHERE case_id=NEW.case_id AND stage_revision=anchor_row.scheduling_stage_revision
            ) stages;
            IF anchor_stage.source_count IS DISTINCT FROM 1::bigint
                OR anchor_stage.stage IS DISTINCT FROM anchor_row.scheduling_stage
                OR anchor_stage.values_digest IS DISTINCT FROM anchor_row.scheduling_stage_digest THEN
                RAISE EXCEPTION 'initial anchor stage source differs' USING ERRCODE='23514';
            END IF;
            anchor_marker:='case:'||NEW.case_id::text||':hearing:'||NEW.anchor_hearing_id::text
                ||':revision:'||anchor_row.revision::text||':operation:'||anchor_row.operation_id::text
                ||':sha256:'||encode(anchor_row.submission_digest,'hex');
            SELECT count(*) INTO source_count FROM (SELECT 1 FROM %1$I.audit_events
                WHERE resource COLLATE "C"=anchor_marker COLLATE "C" LIMIT 2) events;
            IF source_count<>1 THEN
                RAISE EXCEPTION 'initial anchor original audit is missing or duplicated' USING ERRCODE='23514';
            END IF;
            SELECT e.sequence,e.actor,e.action,e.timestamp INTO anchor_audit FROM %1$I.audit_events e
                WHERE e.resource COLLATE "C"=anchor_marker COLLATE "C"
                    AND octet_length(e.timestamp)<=64 AND octet_length(e.actor) BETWEEN 1 AND 1280
                    AND octet_length(e.action)<=64 AND octet_length(e.resource)<=512 AND octet_length(e.chain)=32;
            anchor_time:=to_char(to_timestamp(anchor_row.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
                ||CASE WHEN anchor_row.recorded_at_nanoseconds=0 THEN 'Z'
                    ELSE '.'||rtrim(lpad(anchor_row.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END;
            IF anchor_audit.sequence IS NULL OR anchor_audit.sequence<=anchor_audit_sequence
                OR anchor_audit.actor COLLATE "C" IS DISTINCT FROM anchor_row.recorded_by_email COLLATE "C"
                OR anchor_audit.action COLLATE "C" IS DISTINCT FROM (CASE anchor_row.action
                    WHEN 'schedule' THEN 'hearing.scheduled' WHEN 'replace' THEN 'hearing.replaced'
                    WHEN 'cancel' THEN 'hearing.cancelled' END) COLLATE "C"
                OR anchor_audit.timestamp COLLATE "C" IS DISTINCT FROM anchor_time COLLATE "C" THEN
                RAISE EXCEPTION 'initial anchor original audit differs from its receipt' USING ERRCODE='23514';
            END IF;
            anchor_audit_sequence:=anchor_audit.sequence;
            anchor_previous:=anchor_row;
        END LOOP;
        IF anchor_ordinal<>NEW.anchor_revision
            OR anchor_previous.values_digest IS DISTINCT FROM NEW.anchor_values_digest
            OR anchor_previous.submission_digest IS DISTINCT FROM NEW.anchor_submission_digest
            OR anchor_previous.recorded_administration_revision>NEW.observed_administration_revision
            OR anchor_previous.scheduling_stage_revision>NEW.observed_stage_revision
            OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)
                <ROW(anchor_previous.recorded_at_seconds,anchor_previous.recorded_at_nanoseconds) THEN
            RAISE EXCEPTION 'initial anchor selection context or capture time differs' USING ERRCODE='23514';
        END IF;
    ELSE
        RAISE EXCEPTION 'unsupported measure decision anchor family' USING ERRCODE='23514';
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
            IF jsonb_typeof(effect) IS DISTINCT FROM 'object' THEN
                RAISE EXCEPTION 'measure effect must be an object' USING ERRCODE='23514';
            END IF;
            effect_action:=effect->>'action';
            IF effect_action='impose' THEN
                IF NOT(effect ?& ARRAY['action','proposal']) OR effect-ARRAY['action','proposal']<>'{}'::jsonb THEN
                    RAISE EXCEPTION 'imposition fields differ' USING ERRCODE='23514';
                END IF;
                total:=total+1;
            ELSIF effect_action IN ('confirm','revoke','cease') THEN
                IF NOT(effect ?& ARRAY['action','previous']) OR effect-ARRAY['action','previous']<>'{}'::jsonb THEN
                    RAISE EXCEPTION 'predecessor effect fields differ' USING ERRCODE='23514';
                END IF;
                total:=total+1;
            ELSIF effect_action='modify' THEN
                IF NOT(effect ?& ARRAY['action','previous','values'])
                    OR effect-ARRAY['action','previous','values']<>'{}'::jsonb
                    OR jsonb_typeof(effect->'values') IS DISTINCT FROM 'object' THEN
                    RAISE EXCEPTION 'modification fields differ' USING ERRCODE='23514';
                END IF;
                total:=total+1;
            ELSIF effect_action='substitute' THEN
                IF NOT(effect ?& ARRAY['action','predecessors','successors'])
                    OR effect-ARRAY['action','predecessors','successors']<>'{}'::jsonb
                    OR jsonb_typeof(effect->'predecessors') IS DISTINCT FROM 'array'
                    OR jsonb_typeof(effect->'successors') IS DISTINCT FROM 'array' THEN
                    RAISE EXCEPTION 'substitution fields differ' USING ERRCODE='23514';
                END IF;
                IF jsonb_array_length(effect->'predecessors') NOT BETWEEN 1 AND 32
                    OR jsonb_array_length(effect->'successors') NOT BETWEEN 1 AND 32 THEN
                    RAISE EXCEPTION 'substitution side exceeds bound' USING ERRCODE='23514';
                END IF;
                total:=total+jsonb_array_length(effect->'predecessors')+jsonb_array_length(effect->'successors');
            ELSE
                RAISE EXCEPTION 'unsupported measure effect' USING ERRCODE='23514';
            END IF;
            IF total>32 THEN
                RAISE EXCEPTION 'affected measure identities exceed bound' USING ERRCODE='23514';
            END IF;
        END LOOP;
        FOR effect IN SELECT jsonb_array_elements(value->'effects') LOOP
            effect_action:=effect->>'action'; effect_key:=NULL; subject_key:=NULL;
            FOR side_index IN 0..1 LOOP
                side:=CASE WHEN side_index=0 THEN CASE effect_action
                    WHEN 'impose' THEN '[]'::jsonb WHEN 'substitute' THEN effect->'predecessors'
                    ELSE jsonb_build_array(effect->'previous') END
                    ELSE CASE effect_action WHEN 'impose' THEN jsonb_build_array(effect->'proposal')
                    WHEN 'substitute' THEN effect->'successors' ELSE '[]'::jsonb END END;
                previous_id:=NULL;
                FOR selected IN SELECT jsonb_array_elements(side) LOOP
                    IF jsonb_typeof(selected) IS DISTINCT FROM 'object'
                        OR jsonb_typeof(selected->'id') IS DISTINCT FROM 'string'
                        OR (selected->>'id') !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' THEN
                        RAISE EXCEPTION 'measure identity is not canonical' USING ERRCODE='23514';
                    END IF;
                    selected_id:=(selected->>'id')::uuid;
                    IF selected_id=ANY(all_ids) OR (previous_id IS NOT NULL AND selected_id<=previous_id) THEN
                        RAISE EXCEPTION 'measure identities overlap or are unordered' USING ERRCODE='23514';
                    END IF;
                    all_ids:=array_append(all_ids,selected_id); previous_id:=selected_id;
                    IF effect_key IS NULL OR selected_id<effect_key THEN effect_key:=selected_id; END IF;
                    IF side_index=0 THEN
                        IF NOT(selected ?& ARRAY['id','revision','digest'])
                            OR selected-ARRAY['id','revision','digest']<>'{}'::jsonb
                            OR jsonb_typeof(selected->'revision') IS DISTINCT FROM 'number'
                            OR (selected->>'revision') !~ '^[1-9][0-9]{0,9}$'
                            OR jsonb_typeof(selected->'digest') IS DISTINCT FROM 'string'
                            OR (selected->>'digest') !~ '^[0-9a-f]{64}$' THEN
                            RAISE EXCEPTION 'measure predecessor is not canonical' USING ERRCODE='23514';
                        END IF;
                        old_revision:=(selected->>'revision')::bigint;
                        IF old_revision>=4294967295 THEN
                            RAISE EXCEPTION 'measure revision cannot advance' USING ERRCODE='23514';
                        END IF;
                        SELECT r.*,d.recorded_at_seconds,d.recorded_at_nanoseconds INTO prior
                            FROM %1$I.case_measure_revisions r JOIN %1$I.case_measure_decisions d
                            ON d.operation_id=r.owner_operation AND d.case_id=r.case_id
                            JOIN %1$I.case_measures m ON m.id=r.measure_id AND m.case_id=r.case_id
                            WHERE r.measure_id=selected_id AND r.revision=old_revision;
                        IF prior.case_id IS DISTINCT FROM NEW.case_id OR prior.family IS DISTINCT FROM 'm1'
                            OR prior.capture_digest IS DISTINCT FROM decode(selected->>'digest','hex')
                            OR prior.action IN ('revoke','cease','substitute_out')
                            OR old_revision IS DISTINCT FROM (SELECT max(revision) FROM %1$I.case_measure_revisions WHERE measure_id=selected_id)
                            OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(prior.recorded_at_seconds,prior.recorded_at_nanoseconds) THEN
                            RAISE EXCEPTION 'measure predecessor is absent stale terminal or future' USING ERRCODE='23514';
                        END IF;
                        IF EXISTS(SELECT 1 FROM %1$I.case_measure_decisions d WHERE
                            jsonb_path_exists(d.outcome_view,
                                '$.effects[*] ? (@.previous.id == $id && @.previous.revision >= $revision)',
                                jsonb_build_object('id',selected_id::text,'revision',old_revision))
                            OR jsonb_path_exists(d.outcome_view,
                                '$.effects[*].predecessors[*] ? (@.id == $id && @.revision >= $revision)',
                                jsonb_build_object('id',selected_id::text,'revision',old_revision))) THEN
                            RAISE EXCEPTION 'measure predecessor has an advertised later result' USING ERRCODE='23514';
                        END IF;
                        IF effect_action='modify' AND
                            ((effect->'values'->'subject') IS DISTINCT FROM (prior.values_view->'subject')
                            OR (effect->'values'->'kind') IS DISTINCT FROM (prior.values_view->'kind')) THEN
                            RAISE EXCEPTION 'modification changes exact subject or class' USING ERRCODE='23514';
                        END IF;
                        IF effect_action='substitute' THEN
                            IF subject_key IS NULL THEN subject_key:=prior.subject_id;
                            ELSIF subject_key IS DISTINCT FROM prior.subject_id THEN
                                RAISE EXCEPTION 'substitution crosses subjects' USING ERRCODE='23514';
                            END IF;
                        END IF;
                    ELSE
                        IF NOT(selected ?& ARRAY['id','values']) OR selected-ARRAY['id','values']<>'{}'::jsonb
                            OR jsonb_typeof(selected->'values') IS DISTINCT FROM 'object' THEN
                            RAISE EXCEPTION 'measure proposal fields differ' USING ERRCODE='23514';
                        END IF;
                        IF EXISTS(SELECT 1 FROM %1$I.case_measures WHERE id=selected_id)
                            OR EXISTS(SELECT 1 FROM %1$I.case_measure_revisions WHERE measure_id=selected_id)
                            OR EXISTS(SELECT 1 FROM %1$I.case_measure_decisions d WHERE
                                d.outcome_view @> jsonb_build_object('kind','changes','effects',jsonb_build_array(jsonb_build_object('proposal',jsonb_build_object('id',selected_id::text))))
                                OR d.outcome_view @> jsonb_build_object('kind','changes','effects',jsonb_build_array(jsonb_build_object('successors',jsonb_build_array(jsonb_build_object('id',selected_id::text)))))) THEN
                            RAISE EXCEPTION 'new measure identity already belongs to a decision' USING ERRCODE='23514';
                        END IF;
                        IF effect_action='substitute' AND subject_key IS DISTINCT FROM (selected->'values'->'subject'->>'id')::uuid THEN
                            RAISE EXCEPTION 'substitution successor crosses subjects' USING ERRCODE='23514';
                        END IF;
                    END IF;
                END LOOP;
            END LOOP;
            IF effect_key IS NULL OR (old_key IS NOT NULL AND effect_key<=old_key) THEN
                RAISE EXCEPTION 'measure effects are not in canonical order' USING ERRCODE='23514';
            END IF;
            old_key:=effect_key;
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
REVOKE ALL ON FUNCTION enforce_measure_decision_capture() FROM PUBLIC;
