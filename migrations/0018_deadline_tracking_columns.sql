-- Tracking observations are separate from the immutable historical calculation.
ALTER TABLE case_deadline_revisions
    ADD COLUMN IF NOT EXISTS tracking_canonical BYTEA,
    ADD COLUMN IF NOT EXISTS observations_canonical BYTEA,
    ADD COLUMN IF NOT EXISTS tracking_administration_revision BIGINT
        GENERATED ALWAYS AS (
            (deadline_tracking(tracking_canonical)->>'administration_revision')::bigint
        ) STORED,
    ADD COLUMN IF NOT EXISTS cause_event_sequence BIGINT
        GENERATED ALWAYS AS (
            (deadline_submission(submission_canonical)->'cause'->'event'->>'sequence')::bigint
        ) STORED,
    ALTER COLUMN recorded_by DROP NOT NULL,
    ALTER COLUMN recorded_by_email DROP NOT NULL;

ALTER TABLE case_deadline_revisions
    DROP CONSTRAINT IF EXISTS deadline_tracking_administration,
    DROP CONSTRAINT IF EXISTS deadline_cause_event,
    ADD CONSTRAINT deadline_tracking_administration
        FOREIGN KEY(case_id,tracking_administration_revision)
        REFERENCES case_administration_revisions(case_id,revision),
    ADD CONSTRAINT deadline_cause_event
        FOREIGN KEY(cause_event_sequence) REFERENCES deadline_source_events(sequence);
