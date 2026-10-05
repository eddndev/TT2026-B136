DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='case_measure_revisions'::regclass
        AND attname='validity' AND NOT attisdropped) THEN
        ALTER TABLE case_measure_revisions ADD COLUMN validity TEXT NOT NULL DEFAULT 'valid';
        ALTER TABLE case_measure_operations DROP CONSTRAINT measure_operation_family;
        ALTER TABLE case_measure_operations ADD CONSTRAINT measure_operation_family
            CHECK(family COLLATE "C" IN ('g1','a1'));
        ALTER TABLE case_measure_revisions DROP CONSTRAINT measure_revision_family;
        ALTER TABLE case_measure_revisions ADD CONSTRAINT measure_revision_family
            CHECK(family COLLATE "C" IN ('m1','c1'));
        ALTER TABLE case_measure_revisions ADD CONSTRAINT measure_revision_validity CHECK(
            (family COLLATE "C"='m1' AND validity COLLATE "C"='valid')
            OR (family COLLATE "C"='c1' AND validity COLLATE "C" IN ('valid','entered_in_error')));
        ALTER TABLE case_measure_revisions ADD CONSTRAINT measure_revision_capture_scope
            UNIQUE(measure_id,revision,case_id,capture_digest);
    END IF;
END; $$;
CREATE TABLE IF NOT EXISTS case_measure_administrations (
    operation_id UUID NOT NULL,
    case_id UUID NOT NULL,
    action TEXT NOT NULL,
    target_measure_id UUID NOT NULL,
    target_revision BIGINT NOT NULL,
    target_capture_digest BYTEA NOT NULL,
    reason TEXT NOT NULL,
    correction_canonical BYTEA,
    correction_view JSONB,
    correction_digest BYTEA,
    observed_administration_revision BIGINT NOT NULL,
    observed_stage_revision BIGINT NOT NULL,
    observed_context_digest BYTEA NOT NULL,
    support_format TEXT NOT NULL,
    support_policy TEXT NOT NULL,
    recorded_by UUID NOT NULL,
    recorded_by_email TEXT NOT NULL,
    recorded_by_role TEXT NOT NULL,
    recorded_at_seconds BIGINT NOT NULL,
    recorded_at_nanoseconds INTEGER NOT NULL,
    submission_digest BYTEA NOT NULL,
    review_digest BYTEA NOT NULL,
    capture_digest BYTEA NOT NULL,
    CONSTRAINT measure_administration_primary PRIMARY KEY(operation_id),
    CONSTRAINT measure_administration_scope UNIQUE(operation_id,case_id,capture_digest),
    CONSTRAINT measure_administration_owner FOREIGN KEY(operation_id,case_id,capture_digest)
        REFERENCES case_measure_operations(operation_id,case_id,owner_digest) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT measure_administration_target FOREIGN KEY(target_measure_id,target_revision,case_id,target_capture_digest)
        REFERENCES case_measure_revisions(measure_id,revision,case_id,capture_digest),
    CONSTRAINT measure_administration_administration FOREIGN KEY(case_id,observed_administration_revision)
        REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT measure_administration_author FOREIGN KEY(recorded_by) REFERENCES users(id),
    CONSTRAINT measure_administration_action CHECK(action COLLATE "C" IN ('correct','entered_in_error')),
    CONSTRAINT measure_administration_target_revision CHECK(target_revision BETWEEN 1 AND 4294967294),
    CONSTRAINT measure_administration_target_digest CHECK(octet_length(target_capture_digest)=32),
    CONSTRAINT measure_administration_reason CHECK(case_administration_text_valid(reason,1000,TRUE)),
    CONSTRAINT measure_administration_correction_shape CHECK(
        (action COLLATE "C"='correct' AND num_nonnulls(correction_canonical,correction_view,correction_digest)=3)
        OR (action COLLATE "C"='entered_in_error' AND num_nonnulls(correction_canonical,correction_view,correction_digest)=0)),
    CONSTRAINT measure_administration_correction_size CHECK(octet_length(correction_canonical) BETWEEN 38 AND 20040
        AND substring(correction_canonical FROM 1 FOR 6)=convert_to('MCVAL1','UTF8')),
    CONSTRAINT measure_administration_correction_view CHECK(jsonb_typeof(correction_view)='object'
        AND octet_length(correction_view::text)<=32768),
    CONSTRAINT measure_administration_correction_hash CHECK(correction_digest=sha256(correction_canonical)),
    CONSTRAINT measure_administration_administration_range CHECK(observed_administration_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT measure_administration_stage_range CHECK(observed_stage_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT measure_administration_context_size CHECK(octet_length(observed_context_digest)=32),
    CONSTRAINT measure_administration_format CHECK(support_format COLLATE "C" IN ('pdf','docx')),
    CONSTRAINT measure_administration_policy CHECK(support_policy COLLATE "C"='pdf_docx_v1'),
    CONSTRAINT measure_administration_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE)),
    CONSTRAINT measure_administration_role CHECK(recorded_by_role COLLATE "C" IN ('owner','litigator')),
    CONSTRAINT measure_administration_seconds CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    CONSTRAINT measure_administration_nanoseconds CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    CONSTRAINT measure_administration_submission_size CHECK(octet_length(submission_digest)=32),
    CONSTRAINT measure_administration_review_size CHECK(octet_length(review_digest)=32),
    CONSTRAINT measure_administration_capture_size CHECK(octet_length(capture_digest)=32)
);
CREATE INDEX IF NOT EXISTS measure_administration_case_order ON case_measure_administrations(case_id,operation_id);
REVOKE ALL ON case_measure_administrations FROM PUBLIC;
