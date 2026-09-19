-- Failure recording does not claim that the cause or unchecked base was valid.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_deadline_worker_attempt()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE work RECORD; previous_number BIGINT; checked %1$I.case_deadline_revisions;
BEGIN
    SELECT * INTO work FROM %1$I.deadline_reevaluation_jobs WHERE id=NEW.job_id;
    IF work.id IS NULL OR EXISTS(SELECT 1 FROM %1$I.deadline_reevaluation_results WHERE job_id=NEW.job_id) THEN
        RAISE EXCEPTION 'deadline failure requires an existing incomplete job' USING ERRCODE='23514'; END IF;
    SELECT max(attempt_number) INTO previous_number FROM %1$I.deadline_reevaluation_attempts WHERE job_id=NEW.job_id;
    IF previous_number=9223372036854775807 THEN
        RAISE EXCEPTION 'deadline attempt counter exhausted' USING ERRCODE='23514'; END IF;
    IF NEW.attempt_number IS DISTINCT FROM coalesce(previous_number,0)+1 THEN
        RAISE EXCEPTION 'deadline attempts require a consecutive durable number' USING ERRCODE='23514'; END IF;
    IF NEW.checked_base_revision IS NOT NULL THEN
        SELECT * INTO checked FROM %1$I.case_deadline_revisions
            WHERE deadline_id=work.deadline_id AND revision=NEW.checked_base_revision;
        IF checked.revision IS NULL OR checked.case_id IS DISTINCT FROM work.case_id
            OR checked.submission_digest IS DISTINCT FROM NEW.checked_base_submission_digest
            OR checked.capture_digest IS DISTINCT FROM NEW.checked_base_capture_digest THEN
            RAISE EXCEPTION 'deadline attempt checked base receipts differ' USING ERRCODE='23514'; END IF;
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS deadline_worker_attempt_lock ON deadline_reevaluation_attempts;
CREATE TRIGGER deadline_worker_attempt_lock BEFORE INSERT ON deadline_reevaluation_attempts
    FOR EACH STATEMENT EXECUTE FUNCTION lock_deadline_dispatch();
DROP TRIGGER IF EXISTS deadline_worker_attempt_insert ON deadline_reevaluation_attempts;
CREATE TRIGGER deadline_worker_attempt_insert BEFORE INSERT ON deadline_reevaluation_attempts
    FOR EACH ROW EXECUTE FUNCTION validate_deadline_worker_attempt();
DROP TRIGGER IF EXISTS deadline_worker_attempt_immutable ON deadline_reevaluation_attempts;
CREATE TRIGGER deadline_worker_attempt_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON deadline_reevaluation_attempts
    FOR EACH STATEMENT EXECUTE FUNCTION preserve_deadline_dispatch();
REVOKE ALL ON FUNCTION validate_deadline_worker_attempt() FROM PUBLIC;
