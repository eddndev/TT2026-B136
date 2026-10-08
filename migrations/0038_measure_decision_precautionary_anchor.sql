DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='case_measure_decisions'::regclass
        AND attname='anchor_capture_digest' AND NOT attisdropped) THEN
        ALTER TABLE case_measure_decisions
            ADD COLUMN anchor_precautionary_hearing_id UUID,
            ADD COLUMN anchor_capture_digest BYTEA;
        ALTER TABLE case_measure_decisions DROP CONSTRAINT measure_decision_anchor_shape;
        ALTER TABLE case_measure_decisions ADD CONSTRAINT measure_decision_anchor_shape CHECK(
            (anchor_kind COLLATE "C"='none'
                AND num_nonnulls(anchor_hearing_id,anchor_revision,anchor_values_digest,anchor_submission_digest,
                    anchor_precautionary_hearing_id,anchor_capture_digest)=0)
            OR (anchor_kind COLLATE "C"='initial'
                AND num_nonnulls(anchor_hearing_id,anchor_revision,anchor_values_digest,anchor_submission_digest)=4
                AND num_nonnulls(anchor_precautionary_hearing_id,anchor_capture_digest)=0
                AND anchor_revision>=1 AND anchor_revision<=4294967295
                AND octet_length(anchor_values_digest)=32 AND octet_length(anchor_submission_digest)=32)
            OR (anchor_kind COLLATE "C"='precautionary'
                AND num_nonnulls(anchor_precautionary_hearing_id,anchor_revision,anchor_capture_digest)=3
                AND num_nonnulls(anchor_hearing_id,anchor_values_digest,anchor_submission_digest)=0
                AND anchor_revision>=1 AND anchor_revision<=4294967295 AND octet_length(anchor_capture_digest)=32));
        ALTER TABLE case_measure_decisions ADD CONSTRAINT measure_decision_anchor_precautionary
            FOREIGN KEY(anchor_precautionary_hearing_id,anchor_revision)
                REFERENCES case_precautionary_hearing_revisions(hearing_id,revision);
    END IF;
END; $$;
