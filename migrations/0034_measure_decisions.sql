CREATE TABLE IF NOT EXISTS case_measure_operations (
    operation_id UUID NOT NULL,
    case_id UUID NOT NULL,
    family TEXT NOT NULL,
    owner_digest BYTEA NOT NULL,
    audit_sequence BIGINT NOT NULL,
    CONSTRAINT measure_operation_primary PRIMARY KEY(operation_id),
    CONSTRAINT measure_operation_scope UNIQUE(operation_id,case_id),
    CONSTRAINT measure_operation_owner UNIQUE(operation_id,case_id,owner_digest),
    CONSTRAINT measure_operation_audit UNIQUE(audit_sequence),
    CONSTRAINT measure_operation_case FOREIGN KEY(case_id) REFERENCES cases(id),
    CONSTRAINT measure_operation_audit_event FOREIGN KEY(audit_sequence)
        REFERENCES audit_events(sequence) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT measure_operation_family CHECK(family COLLATE "C"='g1'),
    CONSTRAINT measure_operation_digest CHECK(octet_length(owner_digest)=32),
    CONSTRAINT measure_operation_sequence CHECK(audit_sequence>=0)
);
CREATE TABLE IF NOT EXISTS case_measure_decisions (
    decision_id UUID NOT NULL,
    operation_id UUID NOT NULL,
    case_id UUID NOT NULL,
    values_canonical BYTEA NOT NULL,
    values_view JSONB NOT NULL,
    values_digest BYTEA NOT NULL,
    outcome_canonical BYTEA NOT NULL,
    outcome_view JSONB NOT NULL,
    outcome_digest BYTEA NOT NULL,
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
    decision_digest BYTEA NOT NULL,
    group_digest BYTEA NOT NULL,
    CONSTRAINT measure_decision_primary PRIMARY KEY(decision_id),
    CONSTRAINT measure_decision_operation UNIQUE(operation_id),
    CONSTRAINT measure_decision_scope UNIQUE(operation_id,case_id,group_digest),
    CONSTRAINT measure_decision_owner FOREIGN KEY(operation_id,case_id,group_digest)
        REFERENCES case_measure_operations(operation_id,case_id,owner_digest) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT measure_decision_administration FOREIGN KEY(case_id,observed_administration_revision)
        REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT measure_decision_author FOREIGN KEY(recorded_by) REFERENCES users(id),
    CONSTRAINT measure_decision_values_size CHECK(octet_length(values_canonical) BETWEEN 80 AND 16076
        AND substring(values_canonical FROM 1 FOR 6)=convert_to('MDVAL1','UTF8')),
    CONSTRAINT measure_decision_values_view CHECK(jsonb_typeof(values_view)='object' AND octet_length(values_view::text)<=32768),
    CONSTRAINT measure_decision_values_hash CHECK(values_digest=sha256(values_canonical)),
    CONSTRAINT measure_decision_outcome_size CHECK(octet_length(outcome_canonical) BETWEEN 11 AND 645450
        AND substring(outcome_canonical FROM 1 FOR 5)=convert_to('MEFX1','UTF8')),
    CONSTRAINT measure_decision_outcome_view CHECK(jsonb_typeof(outcome_view)='object' AND octet_length(outcome_view::text)<=1048576),
    CONSTRAINT measure_decision_outcome_hash CHECK(outcome_digest=sha256(outcome_canonical)),
    CONSTRAINT measure_decision_administration_range CHECK(observed_administration_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT measure_decision_stage_range CHECK(observed_stage_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT measure_decision_context_size CHECK(octet_length(observed_context_digest)=32),
    CONSTRAINT measure_decision_format CHECK(support_format COLLATE "C" IN ('pdf','docx')),
    CONSTRAINT measure_decision_policy CHECK(support_policy COLLATE "C"='pdf_docx_v1'),
    CONSTRAINT measure_decision_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE)),
    CONSTRAINT measure_decision_role CHECK(recorded_by_role COLLATE "C" IN ('owner','litigator')),
    CONSTRAINT measure_decision_seconds CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    CONSTRAINT measure_decision_nanoseconds CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    CONSTRAINT measure_decision_submission_size CHECK(octet_length(submission_digest)=32),
    CONSTRAINT measure_decision_review_size CHECK(octet_length(review_digest)=32),
    CONSTRAINT measure_decision_capture_size CHECK(octet_length(decision_digest)=32),
    CONSTRAINT measure_decision_group_size CHECK(octet_length(group_digest)=32)
);
CREATE TABLE IF NOT EXISTS case_measures (
    id UUID NOT NULL,
    case_id UUID NOT NULL,
    initial_revision BIGINT NOT NULL DEFAULT 1,
    root_operation UUID NOT NULL,
    CONSTRAINT measure_root_primary PRIMARY KEY(id),
    CONSTRAINT measure_root_scope UNIQUE(id,case_id),
    CONSTRAINT measure_root_owner FOREIGN KEY(root_operation,case_id)
        REFERENCES case_measure_operations(operation_id,case_id),
    CONSTRAINT measure_root_initial CHECK(initial_revision=1)
);
CREATE TABLE IF NOT EXISTS case_measure_revisions (
    measure_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    case_id UUID NOT NULL,
    owner_operation UUID NOT NULL,
    family TEXT NOT NULL,
    action TEXT NOT NULL,
    values_canonical BYTEA NOT NULL,
    values_view JSONB NOT NULL,
    values_digest BYTEA NOT NULL,
    capture_digest BYTEA NOT NULL,
    subject_id UUID NOT NULL,
    subject_revision BIGINT NOT NULL,
    subject_values_digest BYTEA NOT NULL,
    supervisor_id UUID,
    supervisor_revision BIGINT,
    CONSTRAINT measure_revision_primary PRIMARY KEY(measure_id,revision),
    CONSTRAINT measure_revision_scope UNIQUE(measure_id,revision,case_id,owner_operation),
    CONSTRAINT measure_revision_root FOREIGN KEY(measure_id,case_id)
        REFERENCES case_measures(id,case_id) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT measure_revision_owner FOREIGN KEY(owner_operation,case_id)
        REFERENCES case_measure_operations(operation_id,case_id) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT measure_revision_subject FOREIGN KEY(subject_id,subject_revision)
        REFERENCES case_subject_revisions(subject_id,revision),
    CONSTRAINT measure_revision_initial CHECK(revision=1),
    CONSTRAINT measure_revision_family CHECK(family COLLATE "C"='m1'),
    CONSTRAINT measure_revision_action CHECK(action COLLATE "C"='impose'),
    CONSTRAINT measure_revision_values_size CHECK(octet_length(values_canonical) BETWEEN 91 AND 20113
        AND substring(values_canonical FROM 1 FOR 5)=convert_to('MEAS1','UTF8')),
    CONSTRAINT measure_revision_values_view CHECK(jsonb_typeof(values_view)='object' AND octet_length(values_view::text)<=32768),
    CONSTRAINT measure_revision_values_hash CHECK(values_digest=sha256(values_canonical)),
    CONSTRAINT measure_revision_capture_size CHECK(octet_length(capture_digest)=32),
    CONSTRAINT measure_revision_subject_revision CHECK(subject_revision BETWEEN 1 AND 4294967295),
    CONSTRAINT measure_revision_subject_digest CHECK(octet_length(subject_values_digest)=32),
    CONSTRAINT measure_revision_supervisor_pair CHECK((supervisor_id IS NULL)=(supervisor_revision IS NULL)),
    CONSTRAINT measure_revision_supervisor_revision CHECK(supervisor_revision BETWEEN 1 AND 4294967295)
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_measure_operations'::regclass
        AND conname='measure_operation_payload') THEN
        ALTER TABLE case_measure_operations ADD CONSTRAINT measure_operation_payload
            FOREIGN KEY(operation_id,case_id,owner_digest)
            REFERENCES case_measure_decisions(operation_id,case_id,group_digest) DEFERRABLE INITIALLY DEFERRED;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_measures'::regclass
        AND conname='measure_root_first_revision') THEN
        ALTER TABLE case_measures ADD CONSTRAINT measure_root_first_revision
            FOREIGN KEY(id,initial_revision,case_id,root_operation)
            REFERENCES case_measure_revisions(measure_id,revision,case_id,owner_operation) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS measure_decision_case_order ON case_measure_decisions(case_id,decision_id);
CREATE INDEX IF NOT EXISTS measure_revision_owner_order ON case_measure_revisions(owner_operation,measure_id,revision);
CREATE INDEX IF NOT EXISTS measure_root_case_order ON case_measures(case_id,id);
REVOKE ALL ON case_measure_operations,case_measure_decisions,case_measures,case_measure_revisions FROM PUBLIC;
