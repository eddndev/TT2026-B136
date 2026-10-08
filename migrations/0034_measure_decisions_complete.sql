DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_measure_decision_complete() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE selected_operation UUID; owner RECORD; decision RECORD; row_count BIGINT; root_count BIGINT;
    expected_count INTEGER; effect JSONB; member RECORD; selected_id UUID;
BEGIN
    selected_operation:=(to_jsonb(NEW)->>CASE TG_TABLE_NAME
        WHEN 'case_measures' THEN 'root_operation'
        WHEN 'case_measure_revisions' THEN 'owner_operation' ELSE 'operation_id' END)::uuid;
    SELECT * INTO owner FROM %1$I.case_measure_operations WHERE operation_id=selected_operation;
    SELECT * INTO decision FROM %1$I.case_measure_decisions WHERE operation_id=selected_operation;
    IF owner.family IS DISTINCT FROM 'g1' OR decision.case_id IS DISTINCT FROM owner.case_id
        OR decision.group_digest IS DISTINCT FROM owner.owner_digest THEN
        RAISE EXCEPTION 'measure operation lacks its exact decision payload' USING ERRCODE='23514';
    END IF;
    SELECT count(*) INTO row_count FROM %1$I.case_measure_revisions WHERE owner_operation=selected_operation;
    SELECT count(*) INTO root_count FROM %1$I.case_measures WHERE root_operation=selected_operation;
    IF row_count>32 OR root_count>32 THEN
        RAISE EXCEPTION 'measure owner exceeds member bound' USING ERRCODE='23514';
    END IF;
    IF decision.outcome_view->>'kind'='no_measure_change' THEN
        IF row_count<>0 OR root_count<>0 THEN
            RAISE EXCEPTION 'no-measure-change owner cannot have measure members' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    IF decision.outcome_view->>'kind' IS DISTINCT FROM 'changes'
        OR jsonb_typeof(decision.outcome_view->'effects') IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION 'measure owner outcome shape differs' USING ERRCODE='23514';
    END IF;
    expected_count:=jsonb_array_length(decision.outcome_view->'effects');
    IF expected_count NOT BETWEEN 1 AND 32 OR row_count<>expected_count OR root_count<>expected_count THEN
        RAISE EXCEPTION 'measure owner has missing or extra members' USING ERRCODE='23514';
    END IF;
    FOR effect IN SELECT jsonb_array_elements(decision.outcome_view->'effects') LOOP
        selected_id:=(effect->'proposal'->>'id')::uuid;
        SELECT * INTO member FROM %1$I.case_measure_revisions
            WHERE measure_id=selected_id AND revision=1 AND owner_operation=selected_operation;
        IF effect->>'action' IS DISTINCT FROM 'impose' OR member.case_id IS DISTINCT FROM owner.case_id
            OR member.family IS DISTINCT FROM 'm1' OR member.action IS DISTINCT FROM 'impose'
            OR member.values_view IS DISTINCT FROM effect->'proposal'->'values'
            OR NOT EXISTS(SELECT 1 FROM %1$I.case_measures
                WHERE id=selected_id AND case_id=owner.case_id AND initial_revision=1 AND root_operation=selected_operation) THEN
            RAISE EXCEPTION 'measure member differs from its owner proposal and root' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_measure_operations','case_measure_decisions','case_measures','case_measure_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='measure_decision_immutable') THEN
            EXECUTE format('CREATE TRIGGER measure_decision_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_measure_decision_history()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='measure_decision_lock') THEN
            EXECUTE format('CREATE TRIGGER measure_decision_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION lock_measure_decision_history()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='measure_decision_complete') THEN
            EXECUTE format('CREATE CONSTRAINT TRIGGER measure_decision_complete AFTER INSERT ON %I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION enforce_measure_decision_complete()',tab);
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_measure_decisions'::regclass AND tgname='measure_decision_capture') THEN
        CREATE TRIGGER measure_decision_capture BEFORE INSERT ON case_measure_decisions
            FOR EACH ROW EXECUTE FUNCTION enforce_measure_decision_capture();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_measure_revisions'::regclass AND tgname='measure_revision_source') THEN
        CREATE TRIGGER measure_revision_source BEFORE INSERT ON case_measure_revisions
            FOR EACH ROW EXECUTE FUNCTION enforce_measure_revision_source();
    END IF;
END; $$;
REVOKE ALL ON FUNCTION enforce_measure_decision_complete() FROM PUBLIC;
