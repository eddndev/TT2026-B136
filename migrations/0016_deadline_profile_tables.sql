-- A profile root fixes its global or case scope and requires a complete first revision.
CREATE TABLE IF NOT EXISTS deadline_profiles (
    id UUID CONSTRAINT deadline_profile_root_primary PRIMARY KEY,
    case_id UUID CONSTRAINT deadline_profile_root_case REFERENCES cases(id),
    initial_revision BIGINT NOT NULL DEFAULT 1 CONSTRAINT deadline_profile_initial_revision CHECK(initial_revision=1)
);
CREATE TABLE IF NOT EXISTS deadline_profile_revisions (
    profile_id UUID NOT NULL,
    revision BIGINT NOT NULL CONSTRAINT deadline_profile_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    definition_canonical BYTEA NOT NULL CONSTRAINT deadline_profile_definition_size CHECK(octet_length(definition_canonical) BETWEEN 202 AND 3261697),
    definition_digest BYTEA NOT NULL CONSTRAINT deadline_profile_definition_hash CHECK(definition_digest=pg_catalog.sha256(definition_canonical)),
    definition_view JSONB GENERATED ALWAYS AS (deadline_profile_definition(definition_canonical)) STORED,
    algorithm SMALLINT NOT NULL CONSTRAINT deadline_profile_algorithm CHECK(algorithm=1),
    operation_id UUID NOT NULL CONSTRAINT deadline_profile_operation_unique UNIQUE,
    action TEXT NOT NULL CONSTRAINT deadline_profile_action CHECK(action COLLATE "C" IN ('publish','replace','retire')),
    status TEXT GENERATED ALWAYS AS (CASE action WHEN 'retire' THEN 'retired' ELSE 'published' END) STORED,
    reason TEXT,
    submission_canonical BYTEA NOT NULL CONSTRAINT deadline_profile_submission_size CHECK(octet_length(submission_canonical) BETWEEN 92 AND 4096),
    submission_digest BYTEA NOT NULL CONSTRAINT deadline_profile_submission_hash CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    submission_view JSONB GENERATED ALWAYS AS (deadline_profile_submission(submission_canonical)) STORED,
    recorded_at_seconds BIGINT NOT NULL CONSTRAINT deadline_profile_recorded_time_range CHECK(recorded_at_seconds BETWEEN -62135596800 AND 253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CONSTRAINT deadline_profile_recorded_nanos_range CHECK(recorded_at_nanoseconds BETWEEN 0 AND 999999999),
    recorded_by UUID NOT NULL,
    recorded_by_email TEXT NOT NULL,
    CONSTRAINT deadline_profile_revision_primary PRIMARY KEY(profile_id,revision),
    CONSTRAINT deadline_profile_revision_root FOREIGN KEY(profile_id) REFERENCES deadline_profiles(id),
    CONSTRAINT deadline_profile_recorded_actor FOREIGN KEY(recorded_by) REFERENCES users(id),
    CONSTRAINT deadline_profile_receipt_projection CHECK(
        recorded_by=(submission_view->>'actor_id')::uuid
        AND operation_id=(submission_view->>'operation_id')::uuid
        AND profile_id=(submission_view->>'profile_id')::uuid
        AND action=submission_view->>'action'
        AND revision-1=(submission_view->>'expected_revision')::bigint
        AND algorithm=(submission_view->>'algorithm')::smallint
        AND definition_digest=decode(submission_view->>'definition_digest','hex')
        AND reason COLLATE "C" IS NOT DISTINCT FROM (submission_view->>'reason') COLLATE "C"),
    CONSTRAINT deadline_profile_reason CHECK((action='publish' AND reason IS NULL)
        OR (action<>'publish' AND case_administration_text_valid(reason,1000,TRUE))),
    CONSTRAINT deadline_profile_actor_email CHECK(case_administration_text_valid(recorded_by_email,320,FALSE))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='deadline_profiles'::regclass AND conname='deadline_profile_first_revision') THEN
        ALTER TABLE deadline_profiles ADD CONSTRAINT deadline_profile_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES deadline_profile_revisions(profile_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
REVOKE ALL ON deadline_profiles,deadline_profile_revisions FROM PUBLIC;
