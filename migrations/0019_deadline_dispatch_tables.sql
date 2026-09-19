-- Durable positions are seeded only when their table is first created.
DO $$ BEGIN
    IF to_regclass('deadline_dispatch_cursor') IS NULL THEN
        CREATE TABLE deadline_dispatch_cursor (
            singleton BOOLEAN CONSTRAINT deadline_dispatch_primary PRIMARY KEY,
            completed_event_sequence BIGINT,
            active_event_sequence BIGINT,
            after_deadline_id UUID,
            bootstrap_after_deadline_id UUID,
            CONSTRAINT deadline_dispatch_singleton CHECK(singleton IS TRUE),
            CONSTRAINT deadline_dispatch_completed FOREIGN KEY(completed_event_sequence)
                REFERENCES deadline_source_events(sequence),
            CONSTRAINT deadline_dispatch_active FOREIGN KEY(active_event_sequence)
                REFERENCES deadline_source_events(sequence),
            CONSTRAINT deadline_dispatch_position CHECK((
                (completed_event_sequence IS NULL OR completed_event_sequence>0)
                AND num_nonnulls(active_event_sequence,after_deadline_id) IN (0,2)
                AND (active_event_sequence IS NULL
                    OR active_event_sequence>coalesce(completed_event_sequence,0))) IS TRUE)
        );
        INSERT INTO deadline_dispatch_cursor(singleton) VALUES(TRUE);
    END IF;
END; $$;
CREATE TABLE IF NOT EXISTS deadline_reevaluation_jobs (
    id UUID CONSTRAINT deadline_job_primary PRIMARY KEY,
    operation_id UUID NOT NULL CONSTRAINT deadline_job_operation UNIQUE,
    deadline_id UUID NOT NULL,
    case_id UUID NOT NULL,
    event_sequence BIGINT,
    bootstrap_policy_version SMALLINT,
    created_at_seconds BIGINT NOT NULL,
    created_at_nanoseconds INTEGER NOT NULL,
    CONSTRAINT deadline_job_root FOREIGN KEY(deadline_id,case_id)
        REFERENCES case_deadlines(id,case_id),
    CONSTRAINT deadline_job_event FOREIGN KEY(event_sequence)
        REFERENCES deadline_source_events(sequence),
    CONSTRAINT deadline_job_cause CHECK((
        (event_sequence IS NOT NULL AND event_sequence>0 AND bootstrap_policy_version IS NULL)
        OR (event_sequence IS NULL AND bootstrap_policy_version=1)) IS TRUE),
    CONSTRAINT deadline_job_seconds CHECK(created_at_seconds BETWEEN -62135596800 AND 253402300799),
    CONSTRAINT deadline_job_nanoseconds CHECK(created_at_nanoseconds BETWEEN 0 AND 999999999)
);
CREATE UNIQUE INDEX IF NOT EXISTS deadline_job_event_unique
    ON deadline_reevaluation_jobs(event_sequence,deadline_id) WHERE event_sequence IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS deadline_job_bootstrap_unique
    ON deadline_reevaluation_jobs(deadline_id,bootstrap_policy_version) WHERE event_sequence IS NULL;
CREATE INDEX IF NOT EXISTS deadline_job_created_order
    ON deadline_reevaluation_jobs(created_at_seconds,created_at_nanoseconds,id);
REVOKE ALL ON deadline_dispatch_cursor,deadline_reevaluation_jobs FROM PUBLIC;
