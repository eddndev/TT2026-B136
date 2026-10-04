-- Resource hearings preserve declared scheduling independently of ordinary stages.
CREATE TABLE IF NOT EXISTS case_resource_hearings (
    id UUID CONSTRAINT resource_hearing_root_primary PRIMARY KEY,
    case_id UUID NOT NULL,
    resource_id UUID NOT NULL,
    initial_revision BIGINT NOT NULL DEFAULT 1 CHECK(initial_revision=1),
    CONSTRAINT resource_hearing_root_scope UNIQUE(id,case_id,resource_id),
    CONSTRAINT resource_hearing_resource_root FOREIGN KEY(resource_id,case_id)
        REFERENCES case_procedural_resources(id,case_id)
);
CREATE TABLE IF NOT EXISTS case_resource_hearing_revisions (
    hearing_id UUID NOT NULL,
    case_id UUID NOT NULL,
    resource_id UUID NOT NULL,
    revision BIGINT NOT NULL CHECK(revision=1),
    operation_id UUID NOT NULL CONSTRAINT resource_hearing_operation UNIQUE,
    association_id UUID NOT NULL,
    resource_revision BIGINT NOT NULL CHECK(resource_revision>=1 AND resource_revision<=4294967295),
    resource_capture_digest BYTEA NOT NULL CHECK(octet_length(resource_capture_digest)=32),
    act_id UUID,
    act_revision BIGINT CHECK(act_revision>=1 AND act_revision<=4294967295),
    act_resource_revision BIGINT CHECK(act_resource_revision>=2 AND act_resource_revision<=4294967295),
    act_capture_digest BYTEA CHECK(octet_length(act_capture_digest)=32),
    recorded_resource_revision BIGINT NOT NULL CHECK(recorded_resource_revision>=1 AND recorded_resource_revision<=4294967295),
    recorded_resource_capture_digest BYTEA NOT NULL CHECK(octet_length(recorded_resource_capture_digest)=32),
    values_view JSONB NOT NULL CHECK(jsonb_typeof(values_view)='object' AND octet_length(values_view::text)<=65536),
    values_canonical BYTEA NOT NULL CHECK(octet_length(values_canonical)>=6 AND octet_length(values_canonical)<=65536),
    submission_canonical BYTEA NOT NULL CHECK(octet_length(submission_canonical)>=5 AND octet_length(submission_canonical)<=1048576),
    submission_digest BYTEA NOT NULL CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    capture_canonical BYTEA NOT NULL CHECK(octet_length(capture_canonical)>=5 AND octet_length(capture_canonical)<=1048576),
    capture_digest BYTEA NOT NULL CHECK(capture_digest=pg_catalog.sha256(capture_canonical)),
    recorded_administration_revision BIGINT,
    recorded_administration_digest BYTEA,
    recorded_administration_title TEXT CHECK(octet_length(recorded_administration_title)<=800),
    recorded_administration_reference TEXT CHECK(octet_length(recorded_administration_reference)<=400),
    recorded_at_seconds BIGINT NOT NULL CHECK(recorded_at_seconds>=-62135596800 AND recorded_at_seconds<=253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CHECK(recorded_at_nanoseconds>=0 AND recorded_at_nanoseconds<=999999999),
    recorded_by UUID NOT NULL CONSTRAINT resource_hearing_author REFERENCES users(id),
    recorded_by_email TEXT NOT NULL CHECK(octet_length(recorded_by_email)>=1 AND octet_length(recorded_by_email)<=1280),
    CONSTRAINT resource_hearing_revision_primary PRIMARY KEY(hearing_id,revision),
    CONSTRAINT resource_hearing_revision_scope UNIQUE(hearing_id,case_id,resource_id,revision),
    CONSTRAINT resource_hearing_revision_root FOREIGN KEY(hearing_id,case_id,resource_id)
        REFERENCES case_resource_hearings(id,case_id,resource_id),
    CONSTRAINT resource_hearing_resource_capture FOREIGN KEY(resource_id,case_id,resource_revision)
        REFERENCES case_procedural_resource_revisions(resource_id,case_id,revision),
    CONSTRAINT resource_hearing_resource_head FOREIGN KEY(resource_id,case_id,recorded_resource_revision)
        REFERENCES case_procedural_resource_revisions(resource_id,case_id,revision),
    CONSTRAINT resource_hearing_act_root FOREIGN KEY(act_id,resource_id,case_id)
        REFERENCES case_procedural_resource_acts(id,resource_id,case_id),
    CONSTRAINT resource_hearing_act_capture FOREIGN KEY(resource_id,case_id,act_resource_revision)
        REFERENCES case_procedural_resource_revisions(resource_id,case_id,revision),
    CONSTRAINT resource_hearing_administration FOREIGN KEY(case_id,recorded_administration_revision)
        REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT resource_hearing_association_root FOREIGN KEY(association_id,case_id,resource_id)
        REFERENCES case_resource_activity_associations(id,case_id,resource_id) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT resource_hearing_administration_shape CHECK(
        (recorded_administration_revision IS NULL AND recorded_administration_digest IS NULL
            AND recorded_administration_title IS NOT NULL AND recorded_administration_reference IS NOT NULL)
        OR (recorded_administration_revision>=1 AND recorded_administration_revision<=4294967295
            AND recorded_administration_revision IS NOT NULL AND recorded_administration_digest IS NOT NULL
            AND octet_length(recorded_administration_digest)=32
            AND recorded_administration_title IS NULL AND recorded_administration_reference IS NULL)),
    CONSTRAINT resource_hearing_act_shape CHECK(num_nonnulls(act_id,act_revision,act_resource_revision,act_capture_digest) IN (0,4))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_resource_hearings'::regclass AND conname='resource_hearing_first_revision') THEN
        ALTER TABLE case_resource_hearings ADD CONSTRAINT resource_hearing_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES case_resource_hearing_revisions(hearing_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS resource_hearing_case_resource_order ON case_resource_hearings(case_id,resource_id,id);
REVOKE ALL ON case_resource_hearings,case_resource_hearing_revisions FROM PUBLIC;
