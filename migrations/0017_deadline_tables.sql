-- A root requires its first complete revision in the same transaction.
CREATE TABLE IF NOT EXISTS case_deadlines (
    id UUID CONSTRAINT deadline_root_primary PRIMARY KEY,
    case_id UUID NOT NULL CONSTRAINT deadline_root_case REFERENCES cases(id),
    initial_revision BIGINT NOT NULL DEFAULT 1 CONSTRAINT deadline_initial_revision CHECK(initial_revision=1),
    CONSTRAINT deadline_root_scope UNIQUE(id,case_id)
);
CREATE TABLE IF NOT EXISTS case_deadline_revisions (
    deadline_id UUID NOT NULL,
    case_id UUID NOT NULL,
    revision BIGINT NOT NULL CONSTRAINT deadline_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    title TEXT NOT NULL CONSTRAINT deadline_title CHECK(case_administration_text_valid(title,200,FALSE)),
    profile_id UUID NOT NULL,
    profile_revision BIGINT NOT NULL,
    input_canonical BYTEA NOT NULL CONSTRAINT deadline_input_size CHECK(octet_length(input_canonical) BETWEEN 48 AND 98897),
    input_view JSONB GENERATED ALWAYS AS (deadline_input_selection(input_canonical)) STORED,
    result_canonical BYTEA NOT NULL CONSTRAINT deadline_result_size CHECK(octet_length(result_canonical)>=5 AND octet_length(result_canonical)<=3000000 AND substring(result_canonical FROM 1 FOR 5)=convert_to('DRES1','UTF8')),
    observed_administration_revision BIGINT,
    observed_administration_canonical BYTEA NOT NULL CONSTRAINT deadline_administration_size CHECK(octet_length(observed_administration_canonical)>=17 AND octet_length(observed_administration_canonical)<=16384 AND substring(observed_administration_canonical FROM 1 FOR 5)=convert_to('CADM1','UTF8')),
    observed_administration_digest BYTEA NOT NULL CONSTRAINT deadline_administration_hash CHECK(observed_administration_digest=pg_catalog.sha256(observed_administration_canonical)),
    responsible_id UUID NOT NULL,
    responsible_email TEXT NOT NULL CONSTRAINT deadline_responsible_email CHECK(case_administration_text_valid(responsible_email,320,FALSE)),
    responsible_role TEXT NOT NULL CONSTRAINT deadline_responsible_role CHECK(responsible_role COLLATE "C" IN ('owner','litigator','paralegal')),
    attention JSONB NOT NULL CONSTRAINT deadline_attention CHECK(deadline_attention_valid(attention)),
    operation_id UUID NOT NULL CONSTRAINT deadline_operation_unique UNIQUE,
    action TEXT NOT NULL CONSTRAINT deadline_action CHECK(action COLLATE "C" IN ('register','correct','set_attention','retire')),
    status TEXT GENERATED ALWAYS AS (CASE action WHEN 'retire' THEN 'retired' ELSE 'active' END) STORED,
    reason TEXT,
    review_canonical BYTEA NOT NULL CONSTRAINT deadline_review_size CHECK(octet_length(review_canonical)>=5 AND octet_length(review_canonical)<=524288 AND substring(review_canonical FROM 1 FOR 5)=convert_to('DLRV1','UTF8')),
    capture_canonical BYTEA NOT NULL CONSTRAINT deadline_capture_size CHECK(octet_length(capture_canonical)>=5 AND octet_length(capture_canonical)<=524288 AND substring(capture_canonical FROM 1 FOR 5)=convert_to('DLST1','UTF8')),
    review_digest BYTEA NOT NULL CONSTRAINT deadline_review_hash CHECK(review_digest=pg_catalog.sha256(review_canonical)),
    capture_digest BYTEA NOT NULL CONSTRAINT deadline_capture_hash CHECK(capture_digest=pg_catalog.sha256(capture_canonical)),
    submission_canonical BYTEA NOT NULL CONSTRAINT deadline_submission_size CHECK(octet_length(submission_canonical) BETWEEN 107 AND 4115),
    submission_digest BYTEA NOT NULL CONSTRAINT deadline_submission_hash CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    submission_view JSONB GENERATED ALWAYS AS (deadline_submission(submission_canonical)) STORED,
    recorded_at_seconds BIGINT NOT NULL CONSTRAINT deadline_recorded_seconds CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CONSTRAINT deadline_recorded_nanos CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    recorded_by UUID NOT NULL,
    recorded_by_email TEXT NOT NULL CONSTRAINT deadline_actor_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE)),
    due_at_seconds BIGINT,
    due_at_nanoseconds INTEGER,
    source_kind TEXT,
    source_id UUID,
    source_revision BIGINT,
    source_head_revision BIGINT,
    source_hearing_id UUID,
    source_parent_resolution_id UUID,
    source_parent_resolution_revision BIGINT,
    source_head_parent_resolution_revision BIGINT,
    calendar_id UUID,
    calendar_revision BIGINT,
    calendar_head_revision BIGINT,
    source_fact_family TEXT GENERATED ALWAYS AS (CASE WHEN source_kind IN ('resolution','notification') THEN source_kind END) STORED,
    source_fact_id UUID GENERATED ALWAYS AS (CASE WHEN source_kind IN ('resolution','notification') THEN source_id END) STORED,
    source_result_id UUID GENERATED ALWAYS AS (CASE WHEN source_kind='hearing_result' THEN source_id END) STORED,
    source_parent_family TEXT GENERATED ALWAYS AS (CASE WHEN source_kind='notification' THEN 'resolution' END) STORED,
    CONSTRAINT deadline_revision_primary PRIMARY KEY(deadline_id,revision),
    CONSTRAINT deadline_revision_root FOREIGN KEY(deadline_id,case_id) REFERENCES case_deadlines(id,case_id),
    CONSTRAINT deadline_profile_revision FOREIGN KEY(profile_id,profile_revision) REFERENCES deadline_profile_revisions(profile_id,revision),
    CONSTRAINT deadline_observed_administration FOREIGN KEY(case_id,observed_administration_revision) REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT deadline_responsible FOREIGN KEY(responsible_id) REFERENCES users(id),
    CONSTRAINT deadline_recorded_actor FOREIGN KEY(recorded_by) REFERENCES users(id),
    CONSTRAINT deadline_source_fact_scope FOREIGN KEY(source_fact_family,source_fact_id,case_id) REFERENCES case_procedural_facts(family,id,case_id),
    CONSTRAINT deadline_source_fact_revision FOREIGN KEY(source_fact_family,source_fact_id,source_revision) REFERENCES case_procedural_fact_revisions(family,id,revision),
    CONSTRAINT deadline_source_fact_head FOREIGN KEY(source_fact_family,source_fact_id,source_head_revision) REFERENCES case_procedural_fact_revisions(family,id,revision),
    CONSTRAINT deadline_source_result_scope FOREIGN KEY(source_result_id,case_id,source_hearing_id) REFERENCES case_hearing_results(id,case_id,hearing_id),
    CONSTRAINT deadline_source_result_revision FOREIGN KEY(source_result_id,source_revision) REFERENCES case_hearing_result_revisions(result_id,revision),
    CONSTRAINT deadline_source_result_head FOREIGN KEY(source_result_id,source_head_revision) REFERENCES case_hearing_result_revisions(result_id,revision),
    CONSTRAINT deadline_source_parent_revision FOREIGN KEY(source_parent_family,source_parent_resolution_id,source_parent_resolution_revision) REFERENCES case_procedural_fact_revisions(family,id,revision),
    CONSTRAINT deadline_source_head_parent_revision FOREIGN KEY(source_parent_family,source_parent_resolution_id,source_head_parent_resolution_revision) REFERENCES case_procedural_fact_revisions(family,id,revision),
    CONSTRAINT deadline_calendar_revision FOREIGN KEY(calendar_id,calendar_revision) REFERENCES judicial_calendar_revisions(calendar_id,revision),
    CONSTRAINT deadline_calendar_head FOREIGN KEY(calendar_id,calendar_head_revision) REFERENCES judicial_calendar_revisions(calendar_id,revision),
    CONSTRAINT deadline_due_shape CHECK((due_at_seconds IS NULL AND due_at_nanoseconds IS NULL)
        OR (due_at_seconds IS NOT NULL AND due_at_nanoseconds IS NOT NULL
            AND due_at_seconds BETWEEN -62135596800 AND 253402300799 AND due_at_nanoseconds BETWEEN 0 AND 999999999)),
    CONSTRAINT deadline_source_shape CHECK((source_kind IS NULL AND num_nonnulls(source_id,source_revision,source_head_revision,source_hearing_id,source_parent_resolution_id,source_parent_resolution_revision,source_head_parent_resolution_revision)=0)
        OR (source_kind IS NOT NULL AND source_kind IN ('resolution','notification','hearing_result')
            AND source_id IS NOT NULL AND source_revision IS NOT NULL AND source_head_revision IS NOT NULL
            AND source_revision BETWEEN 1 AND 4294967295 AND source_head_revision BETWEEN source_revision AND 4294967295
            AND ((source_kind='resolution' AND num_nonnulls(source_hearing_id,source_parent_resolution_id,source_parent_resolution_revision,source_head_parent_resolution_revision)=0)
                OR (source_kind='notification' AND source_hearing_id IS NULL AND source_parent_resolution_id IS NOT NULL
                    AND source_parent_resolution_revision IS NOT NULL AND source_head_parent_resolution_revision IS NOT NULL
                    AND source_parent_resolution_revision BETWEEN 1 AND 4294967295 AND source_head_parent_resolution_revision BETWEEN 1 AND 4294967295)
                OR (source_kind='hearing_result' AND source_hearing_id IS NOT NULL
                    AND num_nonnulls(source_parent_resolution_id,source_parent_resolution_revision,source_head_parent_resolution_revision)=0)))),
    CONSTRAINT deadline_calendar_shape CHECK((calendar_id IS NULL AND calendar_revision IS NULL AND calendar_head_revision IS NULL)
        OR (calendar_id IS NOT NULL AND calendar_revision IS NOT NULL AND calendar_head_revision IS NOT NULL
            AND calendar_revision BETWEEN 1 AND 4294967295 AND calendar_head_revision BETWEEN calendar_revision AND 4294967295)),
    CONSTRAINT deadline_input_projection CHECK(case_id=(input_view->>'case_id')::uuid
        AND source_kind IS NOT DISTINCT FROM input_view->>'source_kind'
        AND source_id IS NOT DISTINCT FROM (input_view->>'source_id')::uuid
        AND source_revision IS NOT DISTINCT FROM (input_view->>'source_revision')::bigint
        AND source_hearing_id IS NOT DISTINCT FROM (input_view->>'source_hearing_id')::uuid
        AND source_parent_resolution_id IS NOT DISTINCT FROM (input_view->>'source_parent_resolution_id')::uuid
        AND source_parent_resolution_revision IS NOT DISTINCT FROM (input_view->>'source_parent_resolution_revision')::bigint
        AND calendar_id IS NOT DISTINCT FROM (input_view->>'calendar_id')::uuid
        AND calendar_revision IS NOT DISTINCT FROM (input_view->>'calendar_revision')::bigint),
    CONSTRAINT deadline_receipt_projection CHECK(recorded_by=(submission_view->>'actor_id')::uuid
        AND operation_id=(submission_view->>'operation_id')::uuid AND deadline_id=(submission_view->>'deadline_id')::uuid
        AND case_id=(submission_view->>'case_id')::uuid AND action=submission_view->>'action'
        AND revision-1=(submission_view->>'expected_revision')::bigint
        AND review_digest=decode(submission_view->>'review_digest','hex')
        AND reason COLLATE "C" IS NOT DISTINCT FROM (submission_view->>'reason') COLLATE "C"),
    CONSTRAINT deadline_reason CHECK((action='register' AND reason IS NULL)
        OR (action<>'register' AND reason IS NOT NULL AND case_administration_text_valid(reason,1000,TRUE)))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_deadlines'::regclass AND conname='deadline_first_revision') THEN
        ALTER TABLE case_deadlines ADD CONSTRAINT deadline_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES case_deadline_revisions(deadline_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
REVOKE ALL ON case_deadlines,case_deadline_revisions FROM PUBLIC;
