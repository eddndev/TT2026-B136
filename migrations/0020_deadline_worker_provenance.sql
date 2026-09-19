-- Exact durable admission remains valid after dispatch positions and heads advance.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_worker_cause(job UUID)
RETURNS JSONB LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE work RECORD; event RECORD; source RECORD;
BEGIN
    SELECT * INTO work FROM %1$I.deadline_reevaluation_jobs WHERE id=job;
    IF work.id IS NULL OR NOT EXISTS(SELECT 1 FROM %1$I.case_deadlines
        WHERE id=work.deadline_id AND case_id=work.case_id) THEN
        RAISE EXCEPTION 'deadline worker job or scope is absent' USING ERRCODE='23514'; END IF;
    IF work.event_sequence IS NULL THEN
        IF work.bootstrap_policy_version IS DISTINCT FROM 1 THEN
            RAISE EXCEPTION 'deadline worker bootstrap policy differs' USING ERRCODE='23514'; END IF;
        RETURN jsonb_build_object('kind','legacy_bootstrap','job_id',job,'policy_version',1);
    END IF;
    SELECT * INTO event FROM %1$I.deadline_source_events WHERE sequence=work.event_sequence;
    IF event.sequence IS NULL OR work.bootstrap_policy_version IS NOT NULL
        OR (event.case_id IS NOT NULL AND event.case_id IS DISTINCT FROM work.case_id) THEN
        RAISE EXCEPTION 'deadline worker event or scope differs' USING ERRCODE='23514'; END IF;
    CASE event.source_kind
    WHEN 'resolution','notification' THEN
        SELECT operation_id,case_id,NULL::uuid AS hearing_id INTO source
            FROM %1$I.case_procedural_fact_revisions
            WHERE family=event.source_kind AND id=event.source_id AND revision=event.revision;
    WHEN 'hearing_result' THEN
        SELECT operation_id,case_id,hearing_id INTO source FROM %1$I.case_hearing_result_revisions
            WHERE result_id=event.source_id AND revision=event.revision;
    WHEN 'calendar' THEN
        SELECT operation_id,NULL::uuid AS case_id,NULL::uuid AS hearing_id INTO source
            FROM %1$I.judicial_calendar_revisions WHERE calendar_id=event.source_id AND revision=event.revision;
    WHEN 'profile' THEN
        SELECT r.operation_id,p.case_id,NULL::uuid AS hearing_id INTO source
            FROM %1$I.deadline_profile_revisions r JOIN %1$I.deadline_profiles p ON p.id=r.profile_id
            WHERE r.profile_id=event.source_id AND r.revision=event.revision;
    ELSE
        RAISE EXCEPTION 'deadline worker event family differs' USING ERRCODE='23514';
    END CASE;
    IF source.operation_id IS NULL OR source.operation_id IS DISTINCT FROM event.operation_id
        OR source.case_id IS DISTINCT FROM event.case_id OR source.hearing_id IS DISTINCT FROM event.hearing_id THEN
        RAISE EXCEPTION 'deadline worker event does not match its exact source' USING ERRCODE='23514'; END IF;
    RETURN jsonb_build_object('kind','source_event','job_id',job,'event',jsonb_build_object(
        'sequence',event.sequence,'family',event.source_kind,'source_id',event.source_id,
        'revision',event.revision,'case_id',event.case_id,'hearing_id',event.hearing_id,'operation_id',event.operation_id));
END; $$;
CREATE OR REPLACE FUNCTION %1$I.deadline_worker_authorized(
    scoped_case UUID, deadline UUID, operation UUID, receipt JSONB)
RETURNS UUID LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE work RECORD; exact_cause JSONB;
BEGIN
    SELECT * INTO work FROM %1$I.deadline_reevaluation_jobs WHERE id=(receipt->'cause'->>'job_id')::uuid;
    IF work.id IS NULL OR work.case_id IS DISTINCT FROM scoped_case
        OR work.deadline_id IS DISTINCT FROM deadline OR work.operation_id IS DISTINCT FROM operation
        OR receipt->>'action' IS DISTINCT FROM 'reevaluate'
        OR (receipt->>'case_id')::uuid IS DISTINCT FROM scoped_case
        OR (receipt->>'deadline_id')::uuid IS DISTINCT FROM deadline
        OR (receipt->>'operation_id')::uuid IS DISTINCT FROM operation
        OR receipt->'author' IS DISTINCT FROM jsonb_build_object(
            'kind','technical','service','deadline_reevaluator','policy_version',1) THEN
        RAISE EXCEPTION 'deadline technical transition is not authorized by its job' USING ERRCODE='23514'; END IF;
    exact_cause:=%1$I.deadline_worker_cause(work.id);
    IF receipt->'cause' IS DISTINCT FROM exact_cause THEN
        RAISE EXCEPTION 'deadline technical cause differs from durable admission' USING ERRCODE='23514'; END IF;
    RETURN work.id;
END; $$;
CREATE OR REPLACE FUNCTION %1$I.reserve_deadline_job_operation()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE receipt JSONB;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'deadline writes require read committed isolation' USING ERRCODE='23514'; END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    IF EXISTS(SELECT 1 FROM %1$I.deadline_reevaluation_jobs WHERE operation_id=NEW.operation_id) THEN
        receipt:=%1$I.deadline_submission(NEW.submission_canonical);
        IF NEW.action IS DISTINCT FROM 'reevaluate' OR receipt->'author'->>'kind' IS DISTINCT FROM 'technical' THEN
            RAISE EXCEPTION 'deadline operation is reserved by a durable job'
                USING ERRCODE='23505', CONSTRAINT='deadline_job_operation'; END IF;
        PERFORM %1$I.deadline_worker_authorized(NEW.case_id,NEW.deadline_id,NEW.operation_id,receipt);
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_worker_cause(UUID),
    deadline_worker_authorized(UUID,UUID,UUID,JSONB),reserve_deadline_job_operation() FROM PUBLIC;
