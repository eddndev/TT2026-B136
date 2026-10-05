CREATE TABLE IF NOT EXISTS case_precautionary_hearings (
    id UUID NOT NULL,
    case_id UUID NOT NULL,
    initial_revision BIGINT NOT NULL DEFAULT 1,
    CONSTRAINT precautionary_hearing_root_primary PRIMARY KEY(id),
    CONSTRAINT precautionary_hearing_root_scope UNIQUE(id,case_id),
    CONSTRAINT precautionary_hearing_case FOREIGN KEY(case_id) REFERENCES cases(id),
    CONSTRAINT precautionary_hearing_initial CHECK(initial_revision=1)
);
CREATE TABLE IF NOT EXISTS case_precautionary_hearing_revisions (
    hearing_id UUID NOT NULL,
    case_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    operation_id UUID NOT NULL,
    action TEXT NOT NULL,
    previous_capture_digest BYTEA,
    reason TEXT,
    values_canonical BYTEA,
    values_view JSONB,
    values_digest BYTEA,
    observed_administration_revision BIGINT NOT NULL,
    observed_stage_revision BIGINT NOT NULL,
    observed_context_digest BYTEA NOT NULL,
    support_format TEXT,
    support_policy TEXT,
    recorded_by UUID NOT NULL,
    recorded_by_email TEXT NOT NULL,
    recorded_by_role TEXT NOT NULL,
    recorded_at_seconds BIGINT NOT NULL,
    recorded_at_nanoseconds INTEGER NOT NULL,
    submission_digest BYTEA NOT NULL,
    review_digest BYTEA NOT NULL,
    capture_digest BYTEA NOT NULL,
    audit_sequence BIGINT NOT NULL,
    CONSTRAINT precautionary_hearing_revision_primary PRIMARY KEY(hearing_id,revision),
    CONSTRAINT precautionary_hearing_revision_scope UNIQUE(hearing_id,case_id,revision),
    CONSTRAINT precautionary_hearing_operation UNIQUE(operation_id),
    CONSTRAINT precautionary_hearing_audit UNIQUE(audit_sequence),
    CONSTRAINT precautionary_hearing_revision_root FOREIGN KEY(hearing_id,case_id)
        REFERENCES case_precautionary_hearings(id,case_id),
    CONSTRAINT precautionary_hearing_administration FOREIGN KEY(case_id,observed_administration_revision)
        REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT precautionary_hearing_author FOREIGN KEY(recorded_by) REFERENCES users(id),
    CONSTRAINT precautionary_hearing_audit_event FOREIGN KEY(audit_sequence)
        REFERENCES audit_events(sequence) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT precautionary_hearing_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    CONSTRAINT precautionary_hearing_action CHECK(action COLLATE "C" IN ('schedule','replace','cancel')),
    CONSTRAINT precautionary_hearing_predecessor_size CHECK(octet_length(previous_capture_digest)=32),
    CONSTRAINT precautionary_hearing_reason CHECK(reason IS NULL OR case_administration_text_valid(reason,1000,TRUE)),
    CONSTRAINT precautionary_hearing_command_shape CHECK(
        (action='schedule' AND revision=1 AND previous_capture_digest IS NULL AND reason IS NULL)
        OR (action IN ('replace','cancel') AND revision>1 AND previous_capture_digest IS NOT NULL AND reason IS NOT NULL)),
    CONSTRAINT precautionary_hearing_values_presence CHECK((action='cancel')=(values_canonical IS NULL)),
    CONSTRAINT precautionary_hearing_view_presence CHECK((values_canonical IS NULL)=(values_view IS NULL)),
    CONSTRAINT precautionary_hearing_values_digest_presence CHECK((values_canonical IS NULL)=(values_digest IS NULL)),
    CONSTRAINT precautionary_hearing_format_presence CHECK((values_canonical IS NULL)=(support_format IS NULL)),
    CONSTRAINT precautionary_hearing_policy_presence CHECK((values_canonical IS NULL)=(support_policy IS NULL)),
    CONSTRAINT precautionary_hearing_values_size CHECK(octet_length(values_canonical) BETWEEN 6 AND 16395
        AND substring(values_canonical FROM 1 FOR 6)=convert_to('PHEAR1','UTF8')),
    CONSTRAINT precautionary_hearing_view_size CHECK(jsonb_typeof(values_view)='object' AND octet_length(values_view::text)<=65536),
    CONSTRAINT precautionary_hearing_values_hash CHECK(values_digest=sha256(values_canonical)),
    CONSTRAINT precautionary_hearing_administration_range CHECK(observed_administration_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT precautionary_hearing_stage_range CHECK(observed_stage_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT precautionary_hearing_context_size CHECK(octet_length(observed_context_digest)=32),
    CONSTRAINT precautionary_hearing_format CHECK(support_format COLLATE "C" IN ('pdf','docx')),
    CONSTRAINT precautionary_hearing_policy CHECK(support_policy COLLATE "C"='pdf_docx_v1'),
    CONSTRAINT precautionary_hearing_actor_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE)),
    CONSTRAINT precautionary_hearing_actor_role CHECK(recorded_by_role COLLATE "C" IN ('owner','litigator')),
    CONSTRAINT precautionary_hearing_seconds CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    CONSTRAINT precautionary_hearing_nanoseconds CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    CONSTRAINT precautionary_hearing_submission_size CHECK(octet_length(submission_digest)=32),
    CONSTRAINT precautionary_hearing_review_size CHECK(octet_length(review_digest)=32),
    CONSTRAINT precautionary_hearing_capture_size CHECK(octet_length(capture_digest)=32),
    CONSTRAINT precautionary_hearing_audit_nonnegative CHECK(audit_sequence>=0)
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_precautionary_hearings'::regclass
        AND conname='precautionary_hearing_first_revision') THEN
        ALTER TABLE case_precautionary_hearings ADD CONSTRAINT precautionary_hearing_first_revision
            FOREIGN KEY(id,initial_revision) REFERENCES case_precautionary_hearing_revisions(hearing_id,revision)
            DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS precautionary_hearing_case_order ON case_precautionary_hearings(case_id,id);
REVOKE ALL ON case_precautionary_hearings,case_precautionary_hearing_revisions FROM PUBLIC;
