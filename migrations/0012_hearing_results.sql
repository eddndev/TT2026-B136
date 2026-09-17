-- A result root fixes its exact programming and optional prior declared result.
CREATE TABLE IF NOT EXISTS case_hearing_results (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id),
    hearing_id UUID NOT NULL,
    initial_revision BIGINT NOT NULL DEFAULT 1 CONSTRAINT hearing_result_initial_revision CHECK(initial_revision=1),
    anchor_revision BIGINT NOT NULL CONSTRAINT hearing_result_anchor_revision CHECK(anchor_revision BETWEEN 1 AND 4294967295),
    anchor_values_digest BYTEA NOT NULL CONSTRAINT hearing_result_anchor_values_size CHECK(octet_length(anchor_values_digest)=32),
    anchor_submission_digest BYTEA NOT NULL CONSTRAINT hearing_result_anchor_receipt_size CHECK(octet_length(anchor_submission_digest)=32),
    continuation_hearing_id UUID,
    continuation_result_id UUID,
    continuation_revision BIGINT,
    continuation_values_digest BYTEA,
    continuation_submission_digest BYTEA,
    CONSTRAINT hearing_result_root_scope UNIQUE(id,case_id,hearing_id),
    CONSTRAINT hearing_result_hearing_scope FOREIGN KEY(hearing_id,case_id) REFERENCES case_hearings(id,case_id),
    CONSTRAINT hearing_result_anchor FOREIGN KEY(hearing_id,anchor_revision) REFERENCES case_hearing_revisions(hearing_id,revision),
    CONSTRAINT hearing_result_continuation_scope FOREIGN KEY(continuation_result_id,case_id,continuation_hearing_id)
        REFERENCES case_hearing_results(id,case_id,hearing_id),
    CONSTRAINT hearing_result_continuation_complete CHECK(
        num_nonnulls(continuation_hearing_id,continuation_result_id,continuation_revision,
            continuation_values_digest,continuation_submission_digest)=0
        OR (num_nonnulls(continuation_hearing_id,continuation_result_id,continuation_revision,
            continuation_values_digest,continuation_submission_digest)=5
            AND continuation_result_id<>id AND continuation_revision BETWEEN 1 AND 4294967295
            AND octet_length(continuation_values_digest)=32 AND octet_length(continuation_submission_digest)=32))
);
CREATE TABLE IF NOT EXISTS case_hearing_result_revisions (
    result_id UUID NOT NULL,
    case_id UUID NOT NULL,
    hearing_id UUID NOT NULL,
    revision BIGINT NOT NULL CONSTRAINT hearing_result_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    values_canonical BYTEA NOT NULL CONSTRAINT hearing_result_values_size CHECK(octet_length(values_canonical) BETWEEN 26 AND 146933),
    values_digest BYTEA NOT NULL CONSTRAINT hearing_result_values_hash CHECK(values_digest=pg_catalog.sha256(values_canonical)),
    values_view JSONB GENERATED ALWAYS AS (hearing_result_values(values_canonical)) STORED,
    operation_id UUID NOT NULL CONSTRAINT hearing_result_operation_unique UNIQUE,
    action TEXT NOT NULL CONSTRAINT hearing_result_action CHECK(action COLLATE "C" IN ('record','correct','withdraw')),
    status TEXT GENERATED ALWAYS AS (CASE action WHEN 'withdraw' THEN 'withdrawn' ELSE 'recorded' END) STORED,
    reason TEXT,
    submission_canonical BYTEA NOT NULL CONSTRAINT hearing_result_submission_size CHECK(octet_length(submission_canonical) BETWEEN 192 AND 4296),
    submission_digest BYTEA NOT NULL CONSTRAINT hearing_result_submission_hash CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    submission_view JSONB GENERATED ALWAYS AS (hearing_result_submission(submission_canonical)) STORED,
    recorded_administration_revision BIGINT NOT NULL,
    recorded_administration_digest BYTEA NOT NULL CONSTRAINT hearing_result_recorded_digest_size CHECK(octet_length(recorded_administration_digest)=32),
    recorded_at_seconds BIGINT NOT NULL CONSTRAINT hearing_result_recorded_time_range CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CONSTRAINT hearing_result_recorded_nanos_range CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    recorded_by UUID NOT NULL REFERENCES users(id),
    recorded_by_email TEXT NOT NULL,
    support_name TEXT,
    support_format TEXT,
    support_policy TEXT,
    PRIMARY KEY(result_id,revision),
    CONSTRAINT hearing_result_revision_root FOREIGN KEY(result_id,case_id,hearing_id) REFERENCES case_hearing_results(id,case_id,hearing_id),
    CONSTRAINT hearing_result_recorded_administration FOREIGN KEY(case_id,recorded_administration_revision) REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT hearing_result_receipt_projection CHECK(
        operation_id=(submission_view->>'operation_id')::uuid
        AND recorded_by=(submission_view->>'actor_id')::uuid
        AND case_id=(submission_view->>'case_id')::uuid
        AND hearing_id=(submission_view->>'hearing_id')::uuid
        AND result_id=(submission_view->>'result_id')::uuid
        AND action=submission_view->>'action'
        AND revision-1=(submission_view->>'expected_revision')::bigint
        AND values_digest=decode(submission_view->>'values_digest','hex')
        AND reason IS NOT DISTINCT FROM submission_view->>'reason'),
    CONSTRAINT hearing_result_reason CHECK((action='record' AND reason IS NULL)
        OR (action<>'record' AND case_administration_text_valid(reason,1000,TRUE))),
    CONSTRAINT hearing_result_actor_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE)),
    CONSTRAINT hearing_result_time_bound CHECK(action='withdraw' OR
        CASE values_view->'event_time'->>'precision'
        WHEN 'date' THEN ((values_view->'event_time'->>'date')::date-DATE '1970-01-01')::bigint*86400
            -(values_view->'event_time'->>'offset_seconds')::integer
        ELSE (values_view->'event_time'->>'seconds')::bigint END <= recorded_at_seconds),
    CONSTRAINT hearing_result_support_capture CHECK(
        (values_view->'provenance'->'support'='null'::jsonb AND num_nonnulls(support_name,support_format,support_policy)=0)
        OR (values_view->'provenance'->'support'<>'null'::jsonb AND num_nonnulls(support_name,support_format,support_policy)=3
            AND octet_length(support_name) BETWEEN 1 AND 128
            AND support_format COLLATE "C" IN ('pdf','docx') AND support_policy='pdf_docx_v1'))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_hearing_results'::regclass AND conname='hearing_result_first_revision') THEN
        ALTER TABLE case_hearing_results ADD CONSTRAINT hearing_result_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES case_hearing_result_revisions(result_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_hearing_results'::regclass AND conname='hearing_result_continuation') THEN
        ALTER TABLE case_hearing_results ADD CONSTRAINT hearing_result_continuation FOREIGN KEY(continuation_result_id,continuation_revision)
            REFERENCES case_hearing_result_revisions(result_id,revision);
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS hearing_result_case_hearing_order ON case_hearing_results(case_id,hearing_id,id);
REVOKE ALL ON case_hearing_results,case_hearing_result_revisions FROM PUBLIC;
