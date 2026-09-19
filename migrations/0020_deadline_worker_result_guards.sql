-- A completion binds the current attempted base and any actual produced revision.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_deadline_worker_result()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE work RECORD; base %1$I.case_deadline_revisions; produced %1$I.case_deadline_revisions;
    cause JSONB; receipt JSONB; tracking JSONB; entries JSONB; matched JSONB; latest BIGINT; authorized UUID;
BEGIN
    SELECT * INTO work FROM %1$I.deadline_reevaluation_jobs WHERE id=NEW.job_id;
    cause:=%1$I.deadline_worker_cause(NEW.job_id);
    SELECT * INTO base FROM %1$I.case_deadline_revisions
        WHERE deadline_id=work.deadline_id AND revision=NEW.base_revision;
    IF base.revision IS NULL OR base.case_id IS DISTINCT FROM work.case_id
        OR base.submission_digest IS DISTINCT FROM NEW.base_submission_digest
        OR base.capture_digest IS DISTINCT FROM NEW.base_capture_digest THEN
        RAISE EXCEPTION 'deadline worker result base receipts differ' USING ERRCODE='23514'; END IF;
    SELECT max(revision) INTO latest FROM %1$I.case_deadline_revisions WHERE deadline_id=work.deadline_id;
    SELECT * INTO produced FROM %1$I.case_deadline_revisions WHERE operation_id=work.operation_id;
    IF NEW.outcome='revision' THEN
        IF produced.revision IS NULL OR produced.deadline_id IS DISTINCT FROM work.deadline_id
            OR produced.case_id IS DISTINCT FROM work.case_id OR produced.revision IS DISTINCT FROM NEW.result_revision
            OR produced.revision IS DISTINCT FROM NEW.base_revision+1 OR produced.revision IS DISTINCT FROM latest
            OR produced.submission_digest IS DISTINCT FROM NEW.result_submission_digest
            OR produced.capture_digest IS DISTINCT FROM NEW.result_capture_digest
            OR produced.action IS DISTINCT FROM 'reevaluate' OR produced.recorded_by IS NOT NULL
            OR produced.recorded_by_email IS NOT NULL OR base.status IS DISTINCT FROM 'active'
            OR ROW(produced.recorded_at_seconds,produced.recorded_at_nanoseconds)
                IS DISTINCT FROM ROW(NEW.completed_at_seconds,NEW.completed_at_nanoseconds) THEN
            RAISE EXCEPTION 'deadline worker result does not identify its actual new head' USING ERRCODE='23514'; END IF;
        receipt:=%1$I.deadline_submission(produced.submission_canonical);
        authorized:=%1$I.deadline_worker_authorized(work.case_id,work.deadline_id,work.operation_id,receipt);
        IF authorized IS DISTINCT FROM NEW.job_id
            OR (receipt->>'expected_revision')::bigint IS DISTINCT FROM NEW.base_revision
            OR decode(receipt->'predecessor'->>'submission_digest','hex') IS DISTINCT FROM NEW.base_submission_digest
            OR decode(receipt->'predecessor'->>'capture_digest','hex') IS DISTINCT FROM NEW.base_capture_digest THEN
            RAISE EXCEPTION 'deadline worker result predecessor or job differs' USING ERRCODE='23514'; END IF;
    ELSE
        IF produced.revision IS NOT NULL OR base.revision IS DISTINCT FROM latest THEN
            RAISE EXCEPTION 'deadline no-change result requires the untouched current base' USING ERRCODE='23514'; END IF;
        IF NEW.outcome='retired' THEN
            IF base.status IS DISTINCT FROM 'retired' THEN
                RAISE EXCEPTION 'deadline retired outcome requires a terminal base' USING ERRCODE='23514'; END IF;
        ELSE
            IF base.status IS DISTINCT FROM 'active' THEN
                RAISE EXCEPTION 'deadline nonterminal outcome requires an active deadline' USING ERRCODE='23514'; END IF;
            IF NEW.outcome='already_initialized' THEN
                tracking:=%1$I.deadline_tracking(base.tracking_canonical);
                IF cause->>'kind' IS DISTINCT FROM 'legacy_bootstrap' OR tracking IS NULL
                    OR tracking->'review'->>'state' NOT IN ('accepted','pending') THEN
                    RAISE EXCEPTION 'deadline initialized outcome requires completed tracking initialization' USING ERRCODE='23514'; END IF;
            ELSIF NEW.outcome IN ('already_observed','dependency_not_selected') THEN
                IF cause->>'kind' IS DISTINCT FROM 'source_event' THEN
                    RAISE EXCEPTION 'deadline observed outcome requires a durable source event' USING ERRCODE='23514'; END IF;
                entries:=%1$I.deadline_worker_observations(base,NEW.checked_observations_canonical);
                PERFORM %1$I.deadline_worker_administration(base,NEW.checked_administration_revision,
                    NEW.checked_administration_evidence_digest);
                SELECT item INTO matched FROM jsonb_array_elements(entries) item
                    WHERE item->>'family'=cause->'event'->>'family'
                        AND item->>'id'=cause->'event'->>'source_id'
                        AND item->'case_id'=cause->'event'->'case_id'
                        AND item->'hearing_id'=cause->'event'->'hearing_id';
                IF (NEW.outcome='dependency_not_selected' AND matched IS NOT NULL)
                    OR (NEW.outcome='already_observed' AND (matched IS NULL
                        OR (matched->>'prior_revision')::bigint IS NULL
                        OR (cause->'event'->>'revision')::bigint>(matched->>'prior_revision')::bigint
                        OR (cause->'event'->>'revision')::bigint>(matched->>'revision')::bigint)) THEN
                    RAISE EXCEPTION 'deadline no-change reason differs from examined evidence' USING ERRCODE='23514'; END IF;
            ELSE
                RAISE EXCEPTION 'unsupported deadline completion outcome' USING ERRCODE='23514';
            END IF;
        END IF;
    END IF;
    RETURN NEW;
END; $$;
CREATE OR REPLACE FUNCTION %1$I.require_deadline_worker_result()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE receipt JSONB; job UUID;
BEGIN
    IF NEW.action<>'reevaluate' THEN RETURN NULL; END IF;
    receipt:=%1$I.deadline_submission(NEW.submission_canonical);
    job:=%1$I.deadline_worker_authorized(NEW.case_id,NEW.deadline_id,NEW.operation_id,receipt);
    IF NOT EXISTS(SELECT 1 FROM %1$I.deadline_reevaluation_results r
        WHERE r.job_id=job AND r.outcome='revision' AND r.base_revision=NEW.revision-1
            AND r.base_submission_digest=decode(receipt->'predecessor'->>'submission_digest','hex')
            AND r.base_capture_digest=decode(receipt->'predecessor'->>'capture_digest','hex')
            AND r.result_revision=NEW.revision AND r.result_submission_digest=NEW.submission_digest
            AND r.result_capture_digest=NEW.capture_digest
            AND r.completed_at_seconds=NEW.recorded_at_seconds AND r.completed_at_nanoseconds=NEW.recorded_at_nanoseconds) THEN
        RAISE EXCEPTION 'deadline technical revision requires its matching durable result' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS deadline_worker_result_lock ON deadline_reevaluation_results;
CREATE TRIGGER deadline_worker_result_lock BEFORE INSERT ON deadline_reevaluation_results
    FOR EACH STATEMENT EXECUTE FUNCTION lock_deadline_dispatch();
DROP TRIGGER IF EXISTS deadline_worker_result_insert ON deadline_reevaluation_results;
CREATE TRIGGER deadline_worker_result_insert BEFORE INSERT ON deadline_reevaluation_results
    FOR EACH ROW EXECUTE FUNCTION validate_deadline_worker_result();
DROP TRIGGER IF EXISTS deadline_worker_result_immutable ON deadline_reevaluation_results;
CREATE TRIGGER deadline_worker_result_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON deadline_reevaluation_results
    FOR EACH STATEMENT EXECUTE FUNCTION preserve_deadline_dispatch();
DROP TRIGGER IF EXISTS deadline_worker_revision_result ON case_deadline_revisions;
CREATE CONSTRAINT TRIGGER deadline_worker_revision_result AFTER INSERT ON case_deadline_revisions
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION require_deadline_worker_result();
REVOKE ALL ON FUNCTION validate_deadline_worker_result(),require_deadline_worker_result() FROM PUBLIC;
