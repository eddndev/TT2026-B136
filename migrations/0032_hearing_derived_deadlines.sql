-- Compound origins occupy separate slots; ordinary deadline sources stay reusable.
CREATE TABLE IF NOT EXISTS case_hearing_derived_deadline_origins (
    operation_id UUID CONSTRAINT hearing_derived_deadline_primary PRIMARY KEY,
    case_id UUID NOT NULL,
    hearing_id UUID NOT NULL,
    result_id UUID NOT NULL,
    result_revision BIGINT NOT NULL
        CONSTRAINT hearing_derived_deadline_result_initial CHECK(result_revision=1),
    deadline_id UUID NOT NULL,
    deadline_revision BIGINT NOT NULL
        CONSTRAINT hearing_derived_deadline_deadline_initial CHECK(deadline_revision=1),
    deadline_operation_id UUID NOT NULL
        CONSTRAINT hearing_derived_deadline_deadline_operation_unique UNIQUE,
    source_event_sequence BIGINT NOT NULL
        CONSTRAINT hearing_derived_deadline_event_positive CHECK(source_event_sequence>0)
        CONSTRAINT hearing_derived_deadline_event_unique UNIQUE,
    actor_id UUID NOT NULL,
    actor_email TEXT NOT NULL
        CONSTRAINT hearing_derived_deadline_actor_email CHECK(case_administration_text_valid(actor_email,320,FALSE)),
    actor_role TEXT NOT NULL
        CONSTRAINT hearing_derived_deadline_actor_role CHECK(actor_role COLLATE "C" IN ('owner','litigator')),
    review_canonical BYTEA NOT NULL
        CONSTRAINT hearing_derived_deadline_review_size CHECK(octet_length(review_canonical)>=5
            AND octet_length(review_canonical)<=1048576
            AND substring(review_canonical FROM 1 FOR 5)=convert_to('HRDL1','UTF8')),
    review_digest BYTEA NOT NULL
        CONSTRAINT hearing_derived_deadline_review_hash CHECK(review_digest=pg_catalog.sha256(review_canonical)),
    capture_canonical BYTEA NOT NULL
        CONSTRAINT hearing_derived_deadline_capture_size CHECK(octet_length(capture_canonical)>=457
            AND octet_length(capture_canonical)<=5808
            AND substring(capture_canonical FROM 1 FOR 5)=convert_to('HRDC1','UTF8')),
    capture_digest BYTEA NOT NULL
        CONSTRAINT hearing_derived_deadline_capture_hash CHECK(capture_digest=pg_catalog.sha256(capture_canonical)),
    audit_sequence BIGINT NOT NULL
        CONSTRAINT hearing_derived_deadline_audit_nonnegative CHECK(audit_sequence>=0)
        CONSTRAINT hearing_derived_deadline_audit_unique UNIQUE,
    CONSTRAINT hearing_derived_deadline_result_unique UNIQUE(result_id,result_revision),
    CONSTRAINT hearing_derived_deadline_deadline_unique UNIQUE(deadline_id,deadline_revision),
    CONSTRAINT hearing_derived_deadline_result_scope FOREIGN KEY(result_id,case_id,hearing_id)
        REFERENCES case_hearing_results(id,case_id,hearing_id),
    CONSTRAINT hearing_derived_deadline_result_revision_fk FOREIGN KEY(result_id,result_revision)
        REFERENCES case_hearing_result_revisions(result_id,revision),
    CONSTRAINT hearing_derived_deadline_result_operation_fk FOREIGN KEY(operation_id)
        REFERENCES case_hearing_result_revisions(operation_id),
    CONSTRAINT hearing_derived_deadline_deadline_scope FOREIGN KEY(deadline_id,case_id)
        REFERENCES case_deadlines(id,case_id),
    CONSTRAINT hearing_derived_deadline_deadline_revision_fk FOREIGN KEY(deadline_id,deadline_revision)
        REFERENCES case_deadline_revisions(deadline_id,revision),
    CONSTRAINT hearing_derived_deadline_deadline_operation_fk FOREIGN KEY(deadline_operation_id)
        REFERENCES case_deadline_revisions(operation_id),
    CONSTRAINT hearing_derived_deadline_event_fk FOREIGN KEY(source_event_sequence)
        REFERENCES deadline_source_events(sequence),
    CONSTRAINT hearing_derived_deadline_actor_fk FOREIGN KEY(actor_id) REFERENCES users(id),
    CONSTRAINT hearing_derived_deadline_audit_fk FOREIGN KEY(audit_sequence) REFERENCES audit_events(sequence)
);
CREATE INDEX IF NOT EXISTS hearing_derived_deadline_case_order
    ON case_hearing_derived_deadline_origins(case_id,result_id);
REVOKE ALL ON case_hearing_derived_deadline_origins FROM PUBLIC;
