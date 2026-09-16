-- Hearing roots and revisions preserve declared appointments and operation receipts.
CREATE TABLE IF NOT EXISTS case_hearings (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id),
    initial_revision BIGINT NOT NULL DEFAULT 1 CONSTRAINT hearing_initial_revision CHECK(initial_revision=1),
    CONSTRAINT hearing_root_scope UNIQUE(id,case_id)
);
CREATE TABLE IF NOT EXISTS case_hearing_revisions (
    hearing_id UUID NOT NULL,
    case_id UUID NOT NULL,
    revision BIGINT NOT NULL CONSTRAINT hearing_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    values_canonical BYTEA NOT NULL CONSTRAINT hearing_values_size CHECK(octet_length(values_canonical) BETWEEN 27 AND 10726),
    values_digest BYTEA NOT NULL CONSTRAINT hearing_values_hash CHECK(values_digest=pg_catalog.sha256(values_canonical)),
    values_view JSONB GENERATED ALWAYS AS (hearing_values(values_canonical)) STORED,
    operation_id UUID NOT NULL CONSTRAINT hearing_operation_unique UNIQUE,
    action TEXT NOT NULL CONSTRAINT hearing_action CHECK(action COLLATE "C" IN ('schedule','replace','cancel')),
    status TEXT GENERATED ALWAYS AS (CASE action WHEN 'cancel' THEN 'cancelled' ELSE 'scheduled' END) STORED,
    reason TEXT,
    submission_canonical BYTEA NOT NULL CONSTRAINT hearing_submission_size CHECK(octet_length(submission_canonical) BETWEEN 108 AND 4120),
    submission_digest BYTEA NOT NULL CONSTRAINT hearing_submission_hash CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    submission_view JSONB GENERATED ALWAYS AS (hearing_submission(submission_canonical)) STORED,
    scheduling_administration_revision BIGINT NOT NULL,
    scheduling_administration_digest BYTEA NOT NULL CONSTRAINT hearing_scheduling_digest_size CHECK(octet_length(scheduling_administration_digest)=32),
    scheduling_stage_revision BIGINT NOT NULL CONSTRAINT hearing_stage_revision_range CHECK(scheduling_stage_revision BETWEEN 1 AND 4294967295),
    scheduling_stage TEXT NOT NULL CONSTRAINT hearing_stage_kind CHECK(scheduling_stage COLLATE "C" IN ('investigation','intermediate','trial')),
    scheduling_stage_digest BYTEA CONSTRAINT hearing_stage_digest_size CHECK(scheduling_stage_digest IS NULL OR octet_length(scheduling_stage_digest)=32),
    recorded_administration_revision BIGINT NOT NULL,
    recorded_administration_digest BYTEA NOT NULL CONSTRAINT hearing_recorded_digest_size CHECK(octet_length(recorded_administration_digest)=32),
    recorded_at_seconds BIGINT NOT NULL CONSTRAINT hearing_recorded_time_range CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CONSTRAINT hearing_recorded_nanoseconds_range CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    recorded_by UUID NOT NULL REFERENCES users(id),
    recorded_by_email TEXT NOT NULL,
    support_name TEXT,
    support_format TEXT,
    support_policy TEXT,
    PRIMARY KEY(hearing_id,revision),
    CONSTRAINT hearing_revision_root FOREIGN KEY(hearing_id,case_id) REFERENCES case_hearings(id,case_id),
    CONSTRAINT hearing_scheduling_administration FOREIGN KEY(case_id,scheduling_administration_revision) REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT hearing_recorded_administration FOREIGN KEY(case_id,recorded_administration_revision) REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT hearing_receipt_projection CHECK(
        operation_id=(submission_view->>'operation_id')::uuid
        AND recorded_by=(submission_view->>'actor_id')::uuid
        AND case_id=(submission_view->>'case_id')::uuid
        AND hearing_id=(submission_view->>'hearing_id')::uuid
        AND action=submission_view->>'action'
        AND revision-1=(submission_view->>'expected_revision')::bigint
        AND values_digest=decode(submission_view->>'values_digest','hex')
        AND reason IS NOT DISTINCT FROM submission_view->>'reason'),
    CONSTRAINT hearing_receipt_context CHECK(action='cancel' OR (
        scheduling_administration_revision=(submission_view->'expected_context'->>'case_revision')::bigint
        AND scheduling_stage_revision=(submission_view->'expected_context'->>'stage_revision')::bigint)),
    CONSTRAINT hearing_context_order CHECK(recorded_administration_revision>=scheduling_administration_revision
        AND (action='cancel' OR recorded_administration_revision=scheduling_administration_revision)
        AND (recorded_administration_revision<>scheduling_administration_revision OR recorded_administration_digest=scheduling_administration_digest)),
    CONSTRAINT hearing_kind_context CHECK(scheduling_stage=CASE values_view->>'kind'
        WHEN 'initial' THEN 'investigation' WHEN 'intermediate' THEN 'intermediate' ELSE 'trial' END),
    CONSTRAINT hearing_reason CHECK((action='schedule' AND reason IS NULL) OR (action<>'schedule' AND case_administration_text_valid(reason,1000,TRUE))),
    CONSTRAINT hearing_actor_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE)),
    CONSTRAINT hearing_support_capture CHECK(
        (values_view->>'kind'<>'sentencing' AND num_nonnulls(support_name,support_format,support_policy)=0)
        OR (values_view->>'kind'='sentencing' AND num_nonnulls(support_name,support_format,support_policy)=3
            AND octet_length(support_name) BETWEEN 1 AND 128
            AND support_format COLLATE "C" IN ('pdf','docx') AND support_policy='pdf_docx_v1'))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_hearings'::regclass AND conname='hearing_first_revision') THEN
        ALTER TABLE case_hearings ADD CONSTRAINT hearing_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES case_hearing_revisions(hearing_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS hearing_case_order ON case_hearings(case_id,id);
REVOKE ALL ON case_hearings,case_hearing_revisions FROM PUBLIC;
