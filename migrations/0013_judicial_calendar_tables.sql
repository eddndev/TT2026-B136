-- Global immutable roots reference their first complete revision at commit.
CREATE TABLE IF NOT EXISTS judicial_calendars (
    id UUID CONSTRAINT judicial_calendar_root_primary PRIMARY KEY,
    initial_revision BIGINT NOT NULL DEFAULT 1 CONSTRAINT judicial_calendar_initial_revision CHECK(initial_revision=1)
);
CREATE TABLE IF NOT EXISTS judicial_calendar_revisions (
    calendar_id UUID NOT NULL,
    revision BIGINT NOT NULL CONSTRAINT judicial_calendar_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    values_canonical BYTEA NOT NULL CONSTRAINT judicial_calendar_values_size CHECK(octet_length(values_canonical) BETWEEN 99 AND 191910),
    values_digest BYTEA NOT NULL CONSTRAINT judicial_calendar_values_hash CHECK(values_digest=pg_catalog.sha256(values_canonical)),
    values_view JSONB GENERATED ALWAYS AS (judicial_calendar_values(values_canonical)) STORED,
    operation_id UUID NOT NULL CONSTRAINT judicial_calendar_operation_unique UNIQUE,
    action TEXT NOT NULL CONSTRAINT judicial_calendar_action CHECK(action COLLATE "C" IN ('publish','replace','retire')),
    status TEXT GENERATED ALWAYS AS (CASE action WHEN 'retire' THEN 'retired' ELSE 'published' END) STORED,
    reason TEXT,
    submission_canonical BYTEA NOT NULL CONSTRAINT judicial_calendar_submission_size CHECK(octet_length(submission_canonical) BETWEEN 91 AND 4095),
    submission_digest BYTEA NOT NULL CONSTRAINT judicial_calendar_submission_hash CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    submission_view JSONB GENERATED ALWAYS AS (judicial_calendar_submission(submission_canonical)) STORED,
    recorded_at_seconds BIGINT NOT NULL CONSTRAINT judicial_calendar_recorded_time_range CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CONSTRAINT judicial_calendar_recorded_nanos_range CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    recorded_by UUID NOT NULL,
    recorded_by_email TEXT NOT NULL,
    CONSTRAINT judicial_calendar_revision_primary PRIMARY KEY(calendar_id,revision),
    CONSTRAINT judicial_calendar_revision_root FOREIGN KEY(calendar_id) REFERENCES judicial_calendars(id),
    CONSTRAINT judicial_calendar_recorded_actor FOREIGN KEY(recorded_by) REFERENCES users(id),
    CONSTRAINT judicial_calendar_receipt_projection CHECK(
        recorded_by=(submission_view->>'actor_id')::uuid
        AND operation_id=(submission_view->>'operation_id')::uuid
        AND calendar_id=(submission_view->>'calendar_id')::uuid
        AND action=submission_view->>'action'
        AND revision-1=(submission_view->>'expected_revision')::bigint
        AND values_digest=decode(submission_view->>'values_digest','hex')
        AND reason COLLATE "C" IS NOT DISTINCT FROM (submission_view->>'reason') COLLATE "C"),
    CONSTRAINT judicial_calendar_reason CHECK((action='publish' AND reason IS NULL)
        OR (action<>'publish' AND case_administration_text_valid(reason,1000,TRUE))),
    CONSTRAINT judicial_calendar_actor_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='judicial_calendars'::regclass AND conname='judicial_calendar_first_revision') THEN
        ALTER TABLE judicial_calendars ADD CONSTRAINT judicial_calendar_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES judicial_calendar_revisions(calendar_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
REVOKE ALL ON judicial_calendars,judicial_calendar_revisions FROM PUBLIC;
