DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_measure_decision_complete() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE selected_operation UUID; owner RECORD; decision RECORD; member RECORD; prior RECORD; administrative RECORD;
    row_count BIGINT; root_count BIGINT; expected_count INTEGER:=0; new_count INTEGER:=0;
    effect JSONB; selected JSONB; side JSONB; side_index INTEGER; action_name TEXT;
    result_action TEXT; selected_id UUID; result_revision BIGINT; expected_values JSONB;
    previous_revision BIGINT; seen UUID[]:=ARRAY[]::uuid[];
BEGIN
    IF TG_TABLE_NAME NOT IN ('case_measure_operations','case_measure_decisions','case_measures',
        'case_measure_revisions','case_measure_administrations') THEN
        RAISE EXCEPTION 'unsupported measure ownership table' USING ERRCODE='23514';
    END IF;
    selected_operation:=(to_jsonb(NEW)->>CASE TG_TABLE_NAME
        WHEN 'case_measures' THEN 'root_operation'
        WHEN 'case_measure_revisions' THEN 'owner_operation' ELSE 'operation_id' END)::uuid;
    SELECT * INTO owner FROM %1$I.case_measure_operations WHERE operation_id=selected_operation;
    SELECT count(*) INTO row_count FROM (SELECT 1 FROM %1$I.case_measure_revisions WHERE owner_operation=selected_operation LIMIT 33) r;
    SELECT count(*) INTO root_count FROM (SELECT 1 FROM %1$I.case_measures WHERE root_operation=selected_operation LIMIT 33) r;
    IF row_count>32 OR root_count>32 THEN
        RAISE EXCEPTION 'measure owner exceeds member bound' USING ERRCODE='23514';
    END IF;
    IF owner.family='a1' THEN
        SELECT * INTO administrative FROM %1$I.case_measure_administrations WHERE operation_id=selected_operation;
        IF administrative.case_id IS DISTINCT FROM owner.case_id
            OR administrative.capture_digest IS DISTINCT FROM owner.owner_digest
            OR EXISTS(SELECT 1 FROM %1$I.case_measure_decisions WHERE operation_id=selected_operation)
            OR row_count<>1 OR root_count<>0 THEN
            RAISE EXCEPTION 'administrative owner requires one exact payload and record without roots' USING ERRCODE='23514';
        END IF;
        SELECT * INTO prior FROM %1$I.case_measure_revisions
            WHERE measure_id=administrative.target_measure_id AND revision=administrative.target_revision;
        SELECT * INTO member FROM %1$I.case_measure_revisions WHERE owner_operation=selected_operation;
        IF prior.case_id IS DISTINCT FROM owner.case_id OR prior.family NOT IN ('m1','c1')
            OR prior.validity IS DISTINCT FROM 'valid'
            OR prior.capture_digest IS DISTINCT FROM administrative.target_capture_digest
            OR prior.revision NOT BETWEEN 1 AND 4294967294
            OR member.case_id IS DISTINCT FROM owner.case_id OR member.family IS DISTINCT FROM 'c1'
            OR member.measure_id IS DISTINCT FROM prior.measure_id OR member.revision IS DISTINCT FROM prior.revision+1
            OR member.action IS DISTINCT FROM prior.action OR member.subject_id IS DISTINCT FROM prior.subject_id
            OR member.subject_revision IS DISTINCT FROM prior.subject_revision
            OR member.subject_values_digest IS DISTINCT FROM prior.subject_values_digest
            OR member.supervisor_id IS DISTINCT FROM prior.supervisor_id
            OR member.supervisor_revision IS DISTINCT FROM prior.supervisor_revision
            OR NOT EXISTS(SELECT 1 FROM %1$I.case_measures root JOIN %1$I.case_measure_revisions first
                ON first.measure_id=root.id AND first.case_id=root.case_id AND first.revision=1
                    AND first.owner_operation=root.root_operation AND first.family='m1' AND first.validity='valid'
                JOIN %1$I.case_measure_operations origin ON origin.operation_id=root.root_operation
                    AND origin.case_id=root.case_id AND origin.family='g1'
                WHERE root.id=prior.measure_id AND root.case_id=owner.case_id AND root.initial_revision=1
                    AND first.action IN ('impose','substitute_in')) THEN
            RAISE EXCEPTION 'administrative result replaces exact target identity or sources' USING ERRCODE='23514';
        END IF;
        IF administrative.action='correct' THEN
            IF (prior.values_view->'validity'->'end'='null'::jsonb)
                IS DISTINCT FROM (administrative.correction_view->'validity'->'end'='null'::jsonb)
                OR prior.values_view->'supervision'->>'kind' NOT IN ('known','unknown') THEN
                RAISE EXCEPTION 'correction changes optional or variant structure' USING ERRCODE='23514';
            END IF;
            expected_values:=prior.values_view||jsonb_build_object(
                'conditions',administrative.correction_view->'conditions','validity',administrative.correction_view->'validity',
                'supervision',(prior.values_view->'supervision')||jsonb_build_object(
                    CASE prior.values_view->'supervision'->>'kind' WHEN 'known' THEN 'statement' ELSE 'reason' END,
                    administrative.correction_view->'supervision_text'));
            IF member.validity IS DISTINCT FROM 'valid' OR expected_values IS NOT DISTINCT FROM prior.values_view
                OR member.values_view IS DISTINCT FROM expected_values THEN
                RAISE EXCEPTION 'correction member differs from exact effective declaration' USING ERRCODE='23514';
            END IF;
        ELSIF administrative.action='entered_in_error' THEN
            IF member.validity IS DISTINCT FROM 'entered_in_error'
                OR member.values_canonical IS DISTINCT FROM prior.values_canonical
                OR member.values_view IS DISTINCT FROM prior.values_view
                OR member.values_digest IS DISTINCT FROM prior.values_digest THEN
                RAISE EXCEPTION 'entered-in-error member changes retained declaration' USING ERRCODE='23514';
            END IF;
        ELSE
            RAISE EXCEPTION 'unsupported administrative action' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    SELECT * INTO decision FROM %1$I.case_measure_decisions WHERE operation_id=selected_operation;
    IF owner.family IS DISTINCT FROM 'g1' OR decision.case_id IS DISTINCT FROM owner.case_id
        OR decision.group_digest IS DISTINCT FROM owner.owner_digest
        OR EXISTS(SELECT 1 FROM %1$I.case_measure_administrations WHERE operation_id=selected_operation) THEN
        RAISE EXCEPTION 'measure operation lacks its exact decision payload' USING ERRCODE='23514';
    END IF;
    IF decision.outcome_view->>'kind'='no_measure_change' THEN
        IF row_count<>0 OR root_count<>0 THEN
            RAISE EXCEPTION 'no-measure-change owner cannot have measure members' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    IF decision.outcome_view->>'kind' IS DISTINCT FROM 'changes'
        OR jsonb_typeof(decision.outcome_view->'effects') IS DISTINCT FROM 'array'
        OR octet_length(decision.outcome_view::text)>1048576 THEN
        RAISE EXCEPTION 'measure owner outcome shape differs' USING ERRCODE='23514';
    END IF;
    IF jsonb_array_length(decision.outcome_view->'effects') NOT BETWEEN 1 AND 32 THEN
        RAISE EXCEPTION 'measure owner effect count exceeds bound' USING ERRCODE='23514';
    END IF;
    FOR effect IN SELECT jsonb_array_elements(decision.outcome_view->'effects') LOOP
        action_name:=effect->>'action';
        IF action_name='substitute' THEN
            IF jsonb_typeof(effect->'predecessors') IS DISTINCT FROM 'array'
                OR jsonb_typeof(effect->'successors') IS DISTINCT FROM 'array' THEN
                RAISE EXCEPTION 'substitution sides must be arrays' USING ERRCODE='23514';
            END IF;
            IF jsonb_array_length(effect->'predecessors') NOT BETWEEN 1 AND 32
                OR jsonb_array_length(effect->'successors') NOT BETWEEN 1 AND 32 THEN
                RAISE EXCEPTION 'substitution side exceeds bound' USING ERRCODE='23514';
            END IF;
            expected_count:=expected_count+jsonb_array_length(effect->'predecessors')+jsonb_array_length(effect->'successors');
        ELSIF action_name IN ('impose','confirm','modify','revoke','cease') THEN
            expected_count:=expected_count+1;
        ELSE
            RAISE EXCEPTION 'unsupported measure effect' USING ERRCODE='23514';
        END IF;
        IF expected_count>32 THEN
            RAISE EXCEPTION 'affected measure identities exceed bound' USING ERRCODE='23514';
        END IF;
    END LOOP;
    IF row_count<>expected_count THEN
        RAISE EXCEPTION 'measure owner has missing or extra members' USING ERRCODE='23514';
    END IF;
    FOR effect IN SELECT jsonb_array_elements(decision.outcome_view->'effects') LOOP
        action_name:=effect->>'action';
        FOR side_index IN 0..1 LOOP
            side:=CASE WHEN side_index=0 THEN CASE action_name
                WHEN 'impose' THEN '[]'::jsonb WHEN 'substitute' THEN effect->'predecessors'
                ELSE jsonb_build_array(effect->'previous') END
                ELSE CASE action_name WHEN 'impose' THEN jsonb_build_array(effect->'proposal')
                WHEN 'substitute' THEN effect->'successors' ELSE '[]'::jsonb END END;
            FOR selected IN SELECT jsonb_array_elements(side) LOOP
                selected_id:=(selected->>'id')::uuid;
                IF selected_id IS NULL OR selected_id=ANY(seen) THEN
                    RAISE EXCEPTION 'measure result identity is missing or repeated' USING ERRCODE='23514';
                END IF;
                seen:=array_append(seen,selected_id);
                IF side_index=0 THEN
                    previous_revision:=(selected->>'revision')::bigint;
                    IF previous_revision IS NULL OR previous_revision NOT BETWEEN 1 AND 4294967294 THEN
                        RAISE EXCEPTION 'measure predecessor cannot advance' USING ERRCODE='23514';
                    END IF;
                    SELECT * INTO prior FROM %1$I.case_measure_revisions
                        WHERE measure_id=selected_id AND revision=previous_revision;
                    IF prior.case_id IS DISTINCT FROM owner.case_id OR prior.family IS DISTINCT FROM 'm1'
                        OR prior.validity IS DISTINCT FROM 'valid'
                        OR prior.capture_digest IS DISTINCT FROM decode(selected->>'digest','hex')
                        OR prior.action IN ('revoke','cease','substitute_out') THEN
                        RAISE EXCEPTION 'measure result has an invalid exact predecessor' USING ERRCODE='23514';
                    END IF;
                    result_revision:=previous_revision+1;
                    result_action:=CASE action_name WHEN 'substitute' THEN 'substitute_out' ELSE action_name END;
                    expected_values:=CASE action_name WHEN 'modify' THEN effect->'values' ELSE prior.values_view END;
                ELSE
                    result_revision:=1;
                    result_action:=CASE action_name WHEN 'substitute' THEN 'substitute_in' ELSE action_name END;
                    expected_values:=selected->'values'; new_count:=new_count+1;
                    IF NOT EXISTS(SELECT 1 FROM %1$I.case_measures WHERE id=selected_id
                        AND case_id=owner.case_id AND initial_revision=1 AND root_operation=selected_operation) THEN
                        RAISE EXCEPTION 'new measure result lacks its original root' USING ERRCODE='23514';
                    END IF;
                END IF;
                SELECT * INTO member FROM %1$I.case_measure_revisions
                    WHERE measure_id=selected_id AND revision=result_revision AND owner_operation=selected_operation;
                IF member.case_id IS DISTINCT FROM owner.case_id OR member.family IS DISTINCT FROM 'm1' OR member.validity IS DISTINCT FROM 'valid'
                    OR member.action IS DISTINCT FROM result_action OR member.values_view IS DISTINCT FROM expected_values
                    OR NOT EXISTS(SELECT 1 FROM %1$I.case_measures root JOIN %1$I.case_measure_revisions first
                        ON first.measure_id=root.id AND first.case_id=root.case_id AND first.revision=1
                            AND first.owner_operation=root.root_operation AND first.family='m1' AND first.validity='valid'
                        JOIN %1$I.case_measure_operations origin ON origin.operation_id=root.root_operation
                            AND origin.case_id=root.case_id AND origin.family='g1'
                        WHERE root.id=selected_id AND root.case_id=owner.case_id AND root.initial_revision=1
                            AND first.action IN ('impose','substitute_in')) THEN
                    RAISE EXCEPTION 'measure member differs from its owner effect or original root' USING ERRCODE='23514';
                END IF;
            END LOOP;
        END LOOP;
    END LOOP;
    IF root_count<>new_count THEN
        RAISE EXCEPTION 'measure owner has missing or extra new roots' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION enforce_measure_decision_complete() FROM PUBLIC;
