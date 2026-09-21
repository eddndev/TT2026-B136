-- Associations keep exact references without changing their independent targets.
CREATE TABLE IF NOT EXISTS case_resource_activity_associations (
    id UUID CONSTRAINT resource_activity_root_primary PRIMARY KEY,
    case_id UUID NOT NULL,
    resource_id UUID NOT NULL,
    initial_revision BIGINT NOT NULL DEFAULT 1 CHECK(initial_revision=1),
    CONSTRAINT resource_activity_root_scope UNIQUE(id,case_id,resource_id),
    CONSTRAINT resource_activity_resource_root FOREIGN KEY(resource_id,case_id)
        REFERENCES case_procedural_resources(id,case_id)
);
CREATE TABLE IF NOT EXISTS case_resource_activity_association_revisions (
    association_id UUID NOT NULL,
    case_id UUID NOT NULL,
    resource_id UUID NOT NULL,
    revision BIGINT NOT NULL CHECK(revision>=1 AND revision<=4294967295),
    operation_id UUID NOT NULL CONSTRAINT resource_activity_operation UNIQUE,
    action TEXT NOT NULL CHECK(action COLLATE "C" IN ('link','unlink')),
    status TEXT NOT NULL CHECK(status COLLATE "C" IN ('linked','unlinked')),
    reason TEXT CHECK(reason IS NULL OR (char_length(reason)>=1 AND char_length(reason)<=1000 AND octet_length(reason)<=4000)),
    resource_revision BIGINT NOT NULL CHECK(resource_revision>=1 AND resource_revision<=4294967295),
    resource_capture_digest BYTEA NOT NULL CHECK(octet_length(resource_capture_digest)=32),
    act_id UUID,
    act_revision BIGINT CHECK(act_revision>=1 AND act_revision<=4294967295),
    act_resource_revision BIGINT CHECK(act_resource_revision>=2 AND act_resource_revision<=4294967295),
    act_capture_digest BYTEA CHECK(octet_length(act_capture_digest)=32),
    target_kind TEXT NOT NULL CHECK(target_kind COLLATE "C" IN ('hearing','deadline')),
    hearing_id UUID,
    hearing_revision BIGINT CHECK(hearing_revision>=1 AND hearing_revision<=4294967295),
    hearing_submission_digest BYTEA CHECK(octet_length(hearing_submission_digest)=32),
    deadline_id UUID,
    deadline_revision BIGINT CHECK(deadline_revision>=1 AND deadline_revision<=4294967295),
    deadline_capture_digest BYTEA CHECK(octet_length(deadline_capture_digest)=32),
    selection_canonical BYTEA NOT NULL CHECK(octet_length(selection_canonical) IN (111,167)),
    submission_canonical BYTEA NOT NULL CHECK(octet_length(submission_canonical)>=5 AND octet_length(submission_canonical)<=1048576),
    submission_digest BYTEA NOT NULL CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    capture_canonical BYTEA NOT NULL CHECK(octet_length(capture_canonical)=49),
    capture_digest BYTEA NOT NULL CHECK(capture_digest=pg_catalog.sha256(capture_canonical)),
    previous_capture_digest BYTEA CHECK(octet_length(previous_capture_digest)=32),
    recorded_resource_revision BIGINT NOT NULL CHECK(recorded_resource_revision>=1 AND recorded_resource_revision<=4294967295),
    recorded_resource_capture_digest BYTEA NOT NULL CHECK(octet_length(recorded_resource_capture_digest)=32),
    recorded_administration_revision BIGINT,
    recorded_administration_digest BYTEA,
    recorded_administration_title TEXT CHECK(octet_length(recorded_administration_title)<=800),
    recorded_administration_reference TEXT CHECK(octet_length(recorded_administration_reference)<=400),
    recorded_at_seconds BIGINT NOT NULL CHECK(recorded_at_seconds>=-62135596800 AND recorded_at_seconds<=253402300799),
    recorded_at_nanoseconds INTEGER NOT NULL CHECK(recorded_at_nanoseconds>=0 AND recorded_at_nanoseconds<=999999999),
    recorded_by UUID NOT NULL CONSTRAINT resource_activity_author REFERENCES users(id),
    recorded_by_email TEXT NOT NULL CHECK(octet_length(recorded_by_email)>=1 AND octet_length(recorded_by_email)<=1280),
    CONSTRAINT resource_activity_revision_primary PRIMARY KEY(association_id,revision),
    CONSTRAINT resource_activity_association_root FOREIGN KEY(association_id,case_id,resource_id)
        REFERENCES case_resource_activity_associations(id,case_id,resource_id),
    CONSTRAINT resource_activity_resource_capture FOREIGN KEY(resource_id,case_id,resource_revision)
        REFERENCES case_procedural_resource_revisions(resource_id,case_id,revision),
    CONSTRAINT resource_activity_resource_head FOREIGN KEY(resource_id,case_id,recorded_resource_revision)
        REFERENCES case_procedural_resource_revisions(resource_id,case_id,revision),
    CONSTRAINT resource_activity_act_root FOREIGN KEY(act_id,resource_id,case_id)
        REFERENCES case_procedural_resource_acts(id,resource_id,case_id),
    CONSTRAINT resource_activity_act_capture FOREIGN KEY(resource_id,case_id,act_resource_revision)
        REFERENCES case_procedural_resource_revisions(resource_id,case_id,revision),
    CONSTRAINT resource_activity_hearing_root FOREIGN KEY(hearing_id,case_id) REFERENCES case_hearings(id,case_id),
    CONSTRAINT resource_activity_hearing_capture FOREIGN KEY(hearing_id,hearing_revision)
        REFERENCES case_hearing_revisions(hearing_id,revision),
    CONSTRAINT resource_activity_deadline_root FOREIGN KEY(deadline_id,case_id) REFERENCES case_deadlines(id,case_id),
    CONSTRAINT resource_activity_deadline_capture FOREIGN KEY(deadline_id,deadline_revision)
        REFERENCES case_deadline_revisions(deadline_id,revision),
    CONSTRAINT resource_activity_administration FOREIGN KEY(case_id,recorded_administration_revision)
        REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT resource_activity_administration_shape CHECK(
        (recorded_administration_revision IS NULL AND recorded_administration_digest IS NULL
            AND recorded_administration_title IS NOT NULL AND recorded_administration_reference IS NOT NULL)
        OR (recorded_administration_revision>=1 AND recorded_administration_revision<=4294967295
            AND recorded_administration_revision IS NOT NULL AND recorded_administration_digest IS NOT NULL
            AND octet_length(recorded_administration_digest)=32
            AND recorded_administration_title IS NULL AND recorded_administration_reference IS NULL)),
    CONSTRAINT resource_activity_act_shape CHECK(num_nonnulls(act_id,act_revision,act_resource_revision,act_capture_digest) IN (0,4)),
    CONSTRAINT resource_activity_target_shape CHECK(
        (target_kind='hearing' AND num_nonnulls(hearing_id,hearing_revision,hearing_submission_digest)=3
            AND num_nonnulls(deadline_id,deadline_revision,deadline_capture_digest)=0)
        OR (target_kind='deadline' AND num_nonnulls(deadline_id,deadline_revision,deadline_capture_digest)=3
            AND num_nonnulls(hearing_id,hearing_revision,hearing_submission_digest)=0)),
    CONSTRAINT resource_activity_action_shape CHECK(
        (action='link' AND status='linked' AND revision=1 AND previous_capture_digest IS NULL AND reason IS NULL)
        OR (action='unlink' AND status='unlinked' AND revision=2 AND previous_capture_digest IS NOT NULL AND reason IS NOT NULL))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_resource_activity_associations'::regclass AND conname='resource_activity_first_revision') THEN
        ALTER TABLE case_resource_activity_associations ADD CONSTRAINT resource_activity_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES case_resource_activity_association_revisions(association_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS resource_activity_case_resource_order ON case_resource_activity_associations(case_id,resource_id,id);
REVOKE ALL ON case_resource_activity_associations,case_resource_activity_association_revisions FROM PUBLIC;
