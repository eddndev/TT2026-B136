-- Enforce retained policies and the limited changes allowed to technical writers.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_worker_review(
    base %1$I.case_deadline_revisions, entries JSONB, tracking JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE old_tracking JSONB; policies JSONB; reasons JSONB; entry JSONB; requirement JSONB;
    dependency TEXT; policy TEXT; reason TEXT; state TEXT; retired BOOLEAN; recalculate BOOLEAN:=FALSE;
BEGIN
    IF base.tracking_canonical IS NULL THEN
        policies:=jsonb_build_object('profile','undetermined','source','undetermined','calendar','undetermined');
        reasons:='[]'::jsonb;
    ELSE
        old_tracking:=%1$I.deadline_tracking(base.tracking_canonical);
        policies:=old_tracking->'policies';reasons:=old_tracking->'review'->'reasons';
    END IF;
    IF tracking->'policies' IS DISTINCT FROM policies THEN
        RAISE EXCEPTION 'deadline worker cannot declare or change human policies' USING ERRCODE='23514'; END IF;
    FOR entry IN SELECT jsonb_array_elements(entries) LOOP
        dependency:=CASE entry->>'role' WHEN 'notification_parent' THEN 'source' ELSE entry->>'role' END;
        policy:=policies->>dependency;
        IF policy='undetermined' THEN
            requirement:=jsonb_build_object('dependency',dependency,'reason','policy_undetermined');
            IF NOT reasons @> jsonb_build_array(requirement) THEN reasons:=reasons||jsonb_build_array(requirement); END IF;
        END IF;
        IF (entry->>'revision')::bigint>(entry->>'observed_revision')::bigint THEN
            retired:=entry->>'status' IS DISTINCT FROM CASE WHEN entry->>'family' IN ('profile','calendar')
                THEN 'published' ELSE 'recorded' END;
            reason:=NULL;
            IF retired THEN reason:='dependency_retired';
            ELSIF policy='undetermined' THEN reason:='policy_undetermined';
            ELSIF policy='follow' AND dependency='calendar' THEN recalculate:=TRUE;
            ELSIF policy='follow' AND dependency='source' THEN reason:='source_changed';
            ELSIF policy='follow' AND dependency='profile' THEN reason:='profile_changed';
            END IF;
            IF reason IS NOT NULL THEN
                requirement:=jsonb_build_object('dependency',dependency,'reason',reason);
                IF NOT reasons @> jsonb_build_array(requirement) THEN reasons:=reasons||jsonb_build_array(requirement); END IF;
            END IF;
        END IF;
    END LOOP;
    state:=CASE WHEN jsonb_array_length(reasons)=0 THEN 'accepted' ELSE 'pending' END;
    IF tracking->'review'->>'state' IS DISTINCT FROM state
        OR jsonb_array_length(tracking->'review'->'reasons') IS DISTINCT FROM jsonb_array_length(reasons)
        OR (tracking->'review'->'reasons' @> reasons) IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'deadline worker review changed preserved or required reasons' USING ERRCODE='23514'; END IF;
    RETURN recalculate AND state='accepted' AND coalesce(old_tracking->'review'->>'state'='accepted',FALSE);
END; $$;
CREATE OR REPLACE FUNCTION %1$I.deadline_worker_calendar_input(before_bytes BYTEA, after_bytes BYTEA, revision BIGINT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=21; tag INTEGER; family INTEGER; item JSONB;
BEGIN
    PERFORM %1$I.deadline_input_selection(before_bytes);
    PERFORM %1$I.deadline_input_selection(after_bytes);
    IF revision IS NULL OR revision NOT BETWEEN 1 AND 4294967295 THEN RETURN FALSE; END IF;
    tag:=get_byte(before_bytes,p);p:=p+1;
    IF tag=0 THEN
        item:=%1$I.procedural_fact_atom(before_bytes,p,'text');p:=(item->>'next')::integer;
    ELSE
        family:=get_byte(before_bytes,p);p:=p+1;
        p:=p+CASE family WHEN 0 THEN 20 WHEN 1 THEN 40 WHEN 2 THEN 36 END;
        IF family=2 THEN
            item:=%1$I.procedural_fact_atom(before_bytes,p,'optional_uuid');p:=(item->>'next')::integer;
        END IF;
    END IF;
    tag:=get_byte(before_bytes,p);p:=p+1;
    IF tag=1 THEN
        p:=p+1;
        item:=%1$I.procedural_fact_time(before_bytes,p,'fact');p:=(item->>'next')::integer;
        item:=%1$I.procedural_fact_atom(before_bytes,p,'text');p:=(item->>'next')::integer;
        item:=%1$I.procedural_fact_atom(before_bytes,p,'label');p:=(item->>'next')::integer;
    END IF;
    IF get_byte(before_bytes,p)<>1 THEN RETURN FALSE; END IF;
    p:=p+17;
    RETURN after_bytes=overlay(before_bytes PLACING substring(int8send(revision) FROM 5 FOR 4) FROM p+1 FOR 4);
END; $$;
CREATE OR REPLACE FUNCTION %1$I.enforce_deadline_technical(value %1$I.case_deadline_revisions)
RETURNS VOID LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE previous %1$I.case_deadline_revisions; receipt JSONB; tracking JSONB; entries JSONB;
    old_tracking JSONB; cause JSONB; matched JSONB; job UUID; recalculate BOOLEAN;
    observed_values BYTEA; calendar_revision BIGINT;
BEGIN
    receipt:=%1$I.deadline_submission(value.submission_canonical);
    job:=%1$I.deadline_worker_authorized(value.case_id,value.deadline_id,value.operation_id,receipt);
    IF value.action IS DISTINCT FROM 'reevaluate' OR value.recorded_by IS NOT NULL OR value.recorded_by_email IS NOT NULL
        OR EXISTS(SELECT 1 FROM %1$I.deadline_reevaluation_results WHERE job_id=job) THEN
        RAISE EXCEPTION 'deadline technical author or pending job differs' USING ERRCODE='23514'; END IF;
    SELECT * INTO previous FROM %1$I.case_deadline_revisions
        WHERE deadline_id=value.deadline_id ORDER BY revision DESC LIMIT 1;
    IF previous.revision IS NULL OR previous.status IS DISTINCT FROM 'active'
        OR previous.case_id IS DISTINCT FROM value.case_id OR value.revision IS DISTINCT FROM previous.revision+1
        OR (receipt->>'expected_revision')::bigint IS DISTINCT FROM previous.revision
        OR decode(receipt->'predecessor'->>'submission_digest','hex') IS DISTINCT FROM previous.submission_digest
        OR decode(receipt->'predecessor'->>'capture_digest','hex') IS DISTINCT FROM previous.capture_digest THEN
        RAISE EXCEPTION 'deadline technical predecessor is not the active exact head' USING ERRCODE='23514'; END IF;
    tracking:=%1$I.deadline_tracking(value.tracking_canonical);
    IF %1$I.deadline_tracking_consistent(value.tracking_canonical,value.observations_canonical) IS DISTINCT FROM TRUE
        OR tracking IS NULL THEN
        RAISE EXCEPTION 'deadline technical tracking commitments differ' USING ERRCODE='23514'; END IF;
    entries:=%1$I.deadline_worker_observations(previous,value.observations_canonical);
    cause:=receipt->'cause';
    IF cause->>'kind'='legacy_bootstrap' THEN
        IF previous.tracking_canonical IS NOT NULL THEN
            old_tracking:=%1$I.deadline_tracking(previous.tracking_canonical);
            IF old_tracking->'review'->>'state' IS DISTINCT FROM 'legacy_undeclared' THEN
                RAISE EXCEPTION 'deadline bootstrap cannot revise initialized tracking' USING ERRCODE='23514'; END IF;
        END IF;
    ELSE
        SELECT item INTO matched FROM jsonb_array_elements(entries) item
            WHERE item->>'family'=cause->'event'->>'family'
                AND item->>'id'=cause->'event'->>'source_id'
                AND item->'case_id'=cause->'event'->'case_id'
                AND item->'hearing_id'=cause->'event'->'hearing_id';
        IF matched IS NULL OR (cause->'event'->>'revision')::bigint>(matched->>'revision')::bigint
            OR (cause->'event'->>'revision')::bigint<=(matched->>'prior_revision')::bigint THEN
            RAISE EXCEPTION 'deadline technical event is unselected, future or already observed' USING ERRCODE='23514'; END IF;
    END IF;
    observed_values:=%1$I.deadline_worker_administration(previous,
        (tracking->>'administration_revision')::bigint,decode(tracking->>'administration_evidence_digest','hex'));
    IF observed_values IS DISTINCT FROM decode(tracking->>'administration_values_digest','hex') THEN
        RAISE EXCEPTION 'deadline technical administrative commitment differs' USING ERRCODE='23514'; END IF;
    recalculate:=%1$I.deadline_worker_review(previous,entries,tracking);
    IF ROW(value.title,value.profile_id,value.profile_revision,value.observed_administration_revision,
        value.observed_administration_canonical,value.observed_administration_digest,
        value.responsible_id,value.responsible_email,value.responsible_role,value.attention,
        value.source_kind,value.source_id,value.source_revision,value.source_head_revision,value.source_hearing_id,
        value.source_parent_resolution_id,value.source_parent_resolution_revision,value.source_head_parent_resolution_revision,
        value.calendar_id) IS DISTINCT FROM ROW(previous.title,previous.profile_id,previous.profile_revision,
        previous.observed_administration_revision,previous.observed_administration_canonical,previous.observed_administration_digest,
        previous.responsible_id,previous.responsible_email,previous.responsible_role,previous.attention,
        previous.source_kind,previous.source_id,previous.source_revision,previous.source_head_revision,previous.source_hearing_id,
        previous.source_parent_resolution_id,previous.source_parent_resolution_revision,previous.source_head_parent_resolution_revision,
        previous.calendar_id) THEN
        RAISE EXCEPTION 'deadline technical revision replaced retained human selections' USING ERRCODE='23514'; END IF;
    IF recalculate THEN
        SELECT (item->>'revision')::bigint INTO calendar_revision
            FROM jsonb_array_elements(entries) item WHERE item->>'role'='calendar';
        IF value.calendar_revision IS DISTINCT FROM calendar_revision
            OR value.calendar_head_revision IS DISTINCT FROM calendar_revision
            OR %1$I.deadline_worker_calendar_input(previous.input_canonical,value.input_canonical,calendar_revision) IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION 'deadline technical calculation changed more than the followed calendar' USING ERRCODE='23514'; END IF;
    ELSIF ROW(value.input_canonical,value.result_canonical,value.due_at_seconds,value.due_at_nanoseconds,
        value.calendar_revision,value.calendar_head_revision) IS DISTINCT FROM ROW(previous.input_canonical,
        previous.result_canonical,previous.due_at_seconds,previous.due_at_nanoseconds,previous.calendar_revision,previous.calendar_head_revision) THEN
        RAISE EXCEPTION 'deadline technical revision changed a retained calculation' USING ERRCODE='23514'; END IF;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_worker_review(case_deadline_revisions,JSONB,JSONB),
    deadline_worker_calendar_input(BYTEA,BYTEA,BIGINT),enforce_deadline_technical(case_deadline_revisions) FROM PUBLIC;
