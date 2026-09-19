-- Statement locking preserves the same lock order as begin_audited.
CREATE OR REPLACE FUNCTION lock_deadline_dispatch()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'deadline dispatch requires read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END; $$;
CREATE OR REPLACE FUNCTION preserve_deadline_dispatch()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'deadline dispatch identities and jobs cannot be removed or rewritten' USING ERRCODE='23514';
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_deadline_reevaluation_job()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE position RECORD; next_event BIGINT;
BEGIN
    SELECT * INTO position FROM %1$I.deadline_dispatch_cursor WHERE singleton;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'deadline dispatch cursor is absent' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.case_deadline_revisions r WHERE r.operation_id=NEW.operation_id) THEN
        RAISE EXCEPTION 'deadline job operation is already committed by a deadline'
            USING ERRCODE='23505', CONSTRAINT='deadline_operation_unique';
    END IF;
    IF NEW.event_sequence IS NOT NULL THEN
        IF NEW.bootstrap_policy_version IS NOT NULL THEN
            RAISE EXCEPTION 'deadline job must have exactly one cause' USING ERRCODE='23514';
        END IF;
        SELECT min(e.sequence) INTO next_event FROM %1$I.deadline_source_events e
            WHERE e.sequence>coalesce(position.completed_event_sequence,0);
        IF NEW.event_sequence IS DISTINCT FROM next_event
            OR (position.active_event_sequence IS NOT NULL
                AND position.active_event_sequence IS DISTINCT FROM next_event)
            OR (position.after_deadline_id IS NOT NULL AND NEW.deadline_id<=position.after_deadline_id) THEN
            RAISE EXCEPTION 'deadline job is outside the current event interval' USING ERRCODE='23514';
        END IF;
    ELSIF NEW.bootstrap_policy_version IS DISTINCT FROM 1
        OR (position.bootstrap_after_deadline_id IS NOT NULL
            AND NEW.deadline_id<=position.bootstrap_after_deadline_id) THEN
        RAISE EXCEPTION 'deadline job is outside the current legacy interval' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.deadline_dispatch_candidates(NEW.event_sequence,NEW.deadline_id,TRUE,NEW.deadline_id,FALSE,1) c
        WHERE c.deadline_id=NEW.deadline_id AND c.case_id=NEW.case_id) THEN
        RAISE EXCEPTION 'deadline job cause does not select the current scoped head' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE OR REPLACE FUNCTION %1$I.reserve_deadline_job_operation()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'deadline writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    IF EXISTS(SELECT 1 FROM %1$I.deadline_reevaluation_jobs j WHERE j.operation_id=NEW.operation_id) THEN
        RAISE EXCEPTION 'deadline operation is reserved by a durable job'
            USING ERRCODE='23505', CONSTRAINT='deadline_job_operation';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS deadline_job_lock ON deadline_reevaluation_jobs;
CREATE TRIGGER deadline_job_lock BEFORE INSERT ON deadline_reevaluation_jobs
    FOR EACH STATEMENT EXECUTE FUNCTION lock_deadline_dispatch();
DROP TRIGGER IF EXISTS deadline_job_insert ON deadline_reevaluation_jobs;
CREATE TRIGGER deadline_job_insert BEFORE INSERT ON deadline_reevaluation_jobs
    FOR EACH ROW EXECUTE FUNCTION validate_deadline_reevaluation_job();
DROP TRIGGER IF EXISTS deadline_job_immutable ON deadline_reevaluation_jobs;
CREATE TRIGGER deadline_job_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON deadline_reevaluation_jobs
    FOR EACH STATEMENT EXECUTE FUNCTION preserve_deadline_dispatch();
DROP TRIGGER IF EXISTS deadline_operation_reserved ON case_deadline_revisions;
CREATE TRIGGER deadline_operation_reserved BEFORE INSERT ON case_deadline_revisions
    FOR EACH ROW EXECUTE FUNCTION reserve_deadline_job_operation();
REVOKE ALL ON FUNCTION lock_deadline_dispatch(),preserve_deadline_dispatch(),
    validate_deadline_reevaluation_job(),reserve_deadline_job_operation() FROM PUBLIC;
