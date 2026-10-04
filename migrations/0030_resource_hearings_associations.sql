-- Extend exact activity selections while retaining existing canonical tags.
ALTER TABLE case_resource_activity_association_revisions
    ADD COLUMN IF NOT EXISTS resource_hearing_id UUID,
    ADD COLUMN IF NOT EXISTS resource_hearing_revision BIGINT
        CONSTRAINT resource_activity_resource_hearing_revision_check CHECK(resource_hearing_revision>=1 AND resource_hearing_revision<=4294967295),
    ADD COLUMN IF NOT EXISTS resource_hearing_capture_digest BYTEA
        CONSTRAINT resource_activity_resource_hearing_digest_check CHECK(octet_length(resource_hearing_capture_digest)=32);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_resource_activity_association_revisions'::regclass AND conname='resource_activity_resource_hearing_root') THEN
        ALTER TABLE case_resource_activity_association_revisions
            DROP CONSTRAINT case_resource_activity_association_revisions_target_kind_check,
            DROP CONSTRAINT resource_activity_target_shape,
            ADD CONSTRAINT case_resource_activity_association_revisions_target_kind_check
                CHECK(target_kind COLLATE "C" IN ('hearing','deadline','resource_hearing')),
            ADD CONSTRAINT resource_activity_target_shape CHECK(
                (target_kind='hearing' AND num_nonnulls(hearing_id,hearing_revision,hearing_submission_digest)=3
                    AND num_nonnulls(deadline_id,deadline_revision,deadline_capture_digest,resource_hearing_id,resource_hearing_revision,resource_hearing_capture_digest)=0)
                OR (target_kind='deadline' AND num_nonnulls(deadline_id,deadline_revision,deadline_capture_digest)=3
                    AND num_nonnulls(hearing_id,hearing_revision,hearing_submission_digest,resource_hearing_id,resource_hearing_revision,resource_hearing_capture_digest)=0)
                OR (target_kind='resource_hearing' AND num_nonnulls(resource_hearing_id,resource_hearing_revision,resource_hearing_capture_digest)=3
                    AND num_nonnulls(hearing_id,hearing_revision,hearing_submission_digest,deadline_id,deadline_revision,deadline_capture_digest)=0)),
            ADD CONSTRAINT resource_activity_resource_hearing_root
                FOREIGN KEY(resource_hearing_id,case_id,resource_id)
                REFERENCES case_resource_hearings(id,case_id,resource_id),
            ADD CONSTRAINT resource_activity_resource_hearing_capture
                FOREIGN KEY(resource_hearing_id,case_id,resource_id,resource_hearing_revision)
                REFERENCES case_resource_hearing_revisions(hearing_id,case_id,resource_id,revision);
    END IF;
END; $$;
