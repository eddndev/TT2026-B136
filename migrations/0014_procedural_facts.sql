-- Exact declarations keep immutable roots and append-only revisions.
CREATE TABLE IF NOT EXISTS case_procedural_facts (
    family TEXT NOT NULL CONSTRAINT procedural_fact_family CHECK(family COLLATE "C" IN ('resolution','notification')),
    id UUID NOT NULL,
    case_id UUID NOT NULL REFERENCES cases(id),
    parent_resolution_id UUID,
    parent_family TEXT GENERATED ALWAYS AS (CASE WHEN parent_resolution_id IS NULL THEN NULL ELSE 'resolution' END) STORED,
    initial_revision BIGINT NOT NULL DEFAULT 1 CONSTRAINT procedural_fact_initial_revision CHECK(initial_revision=1),
    PRIMARY KEY(family,id),
    CONSTRAINT procedural_fact_root_scope UNIQUE(family,id,case_id),
    CONSTRAINT procedural_fact_parent_shape CHECK((family='resolution' AND parent_resolution_id IS NULL)
        OR (family='notification' AND parent_resolution_id IS NOT NULL)),
    CONSTRAINT procedural_fact_parent_scope FOREIGN KEY(parent_family,parent_resolution_id,case_id)
        REFERENCES case_procedural_facts(family,id,case_id)
);
CREATE TABLE IF NOT EXISTS case_procedural_fact_revisions (
    family TEXT NOT NULL,
    id UUID NOT NULL,
    case_id UUID NOT NULL,
    revision BIGINT NOT NULL CONSTRAINT procedural_fact_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    values_canonical BYTEA NOT NULL CONSTRAINT procedural_fact_values_size CHECK(
        (family='resolution' AND octet_length(values_canonical) BETWEEN 27 AND 17700)
        OR (family='notification' AND octet_length(values_canonical) BETWEEN 67 AND 58671)),
    values_digest BYTEA NOT NULL CONSTRAINT procedural_fact_values_hash CHECK(values_digest=pg_catalog.sha256(values_canonical)),
    values_view JSONB GENERATED ALWAYS AS (procedural_fact_values(family,values_canonical)) STORED,
    sources_canonical BYTEA NOT NULL CONSTRAINT procedural_fact_sources_size CHECK(octet_length(sources_canonical) BETWEEN 19 AND 36847),
    sources_digest BYTEA NOT NULL CONSTRAINT procedural_fact_sources_hash CHECK(sources_digest=pg_catalog.sha256(sources_canonical)),
    sources_view JSONB GENERATED ALWAYS AS (procedural_fact_sources(sources_canonical)) STORED,
    operation_id UUID NOT NULL CONSTRAINT procedural_fact_operation_unique UNIQUE,
    action TEXT NOT NULL CONSTRAINT procedural_fact_action CHECK(action COLLATE "C" IN ('record','correct','withdraw')),
    status TEXT GENERATED ALWAYS AS (CASE action WHEN 'withdraw' THEN 'withdrawn' ELSE 'recorded' END) STORED,
    reason TEXT,
    submission_canonical BYTEA NOT NULL CONSTRAINT procedural_fact_submission_size CHECK(
        (family='resolution' AND octet_length(submission_canonical) BETWEEN 141 AND 4145)
        OR (family='notification' AND octet_length(submission_canonical) BETWEEN 157 AND 4161)),
    submission_digest BYTEA NOT NULL CONSTRAINT procedural_fact_submission_hash CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    submission_view JSONB GENERATED ALWAYS AS (procedural_fact_submission(submission_canonical)) STORED,
    recorded_administration_revision BIGINT,
    recorded_administration_digest BYTEA,
    recorded_administration_title TEXT,
    recorded_administration_reference TEXT,
    recorded_at_seconds BIGINT NOT NULL CONSTRAINT procedural_fact_recorded_time_range CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CONSTRAINT procedural_fact_recorded_nanos_range CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    recorded_by UUID NOT NULL REFERENCES users(id),
    recorded_by_email TEXT NOT NULL CONSTRAINT procedural_fact_actor_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE)),
    PRIMARY KEY(family,id,revision),
    CONSTRAINT procedural_fact_revision_root FOREIGN KEY(family,id,case_id) REFERENCES case_procedural_facts(family,id,case_id),
    CONSTRAINT procedural_fact_recorded_administration FOREIGN KEY(case_id,recorded_administration_revision) REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT procedural_fact_administration_shape CHECK(
        (recorded_administration_revision IS NULL AND recorded_administration_digest IS NULL
            AND recorded_administration_title IS NOT NULL AND recorded_administration_reference IS NOT NULL)
        OR (recorded_administration_revision IS NOT NULL AND recorded_administration_revision BETWEEN 1 AND 4294967295 AND recorded_administration_digest IS NOT NULL
            AND octet_length(recorded_administration_digest)=32 AND recorded_administration_title IS NULL
            AND recorded_administration_reference IS NULL)),
    CONSTRAINT procedural_fact_receipt_projection CHECK(
        family=submission_view->>'family' AND id=(submission_view->>'target_id')::uuid
        AND operation_id=(submission_view->>'operation_id')::uuid
        AND recorded_by=(submission_view->>'actor_id')::uuid AND case_id=(submission_view->>'case_id')::uuid
        AND action=submission_view->>'action' AND revision-1=(submission_view->>'expected_revision')::bigint
        AND values_digest=decode(submission_view->>'values_digest','hex')
        AND sources_digest=decode(submission_view->>'sources_digest','hex')
        AND reason IS NOT DISTINCT FROM submission_view->>'reason'),
    CONSTRAINT procedural_fact_reason CHECK((action='record' AND reason IS NULL)
        OR (action<>'record' AND reason IS NOT NULL AND case_administration_text_valid(reason,1000,TRUE)))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_procedural_facts'::regclass AND conname='procedural_fact_first_revision') THEN
        ALTER TABLE case_procedural_facts ADD CONSTRAINT procedural_fact_first_revision FOREIGN KEY(family,id,initial_revision)
            REFERENCES case_procedural_fact_revisions(family,id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS procedural_fact_case_order ON case_procedural_facts(case_id,family,parent_resolution_id,id);
REVOKE ALL ON case_procedural_facts,case_procedural_fact_revisions FROM PUBLIC;
