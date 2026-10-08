ALTER TABLE case_measure_decisions
    ADD COLUMN IF NOT EXISTS anchor_kind TEXT NOT NULL DEFAULT 'none',
    ADD COLUMN IF NOT EXISTS anchor_hearing_id UUID,
    ADD COLUMN IF NOT EXISTS anchor_revision BIGINT,
    ADD COLUMN IF NOT EXISTS anchor_values_digest BYTEA,
    ADD COLUMN IF NOT EXISTS anchor_submission_digest BYTEA;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_measure_decisions'::regclass
        AND conname='measure_decision_anchor_shape') THEN
        ALTER TABLE case_measure_decisions ADD CONSTRAINT measure_decision_anchor_shape CHECK(
            (anchor_kind COLLATE "C"='none'
                AND num_nonnulls(anchor_hearing_id,anchor_revision,anchor_values_digest,anchor_submission_digest)=0)
            OR (anchor_kind COLLATE "C"='initial'
                AND num_nonnulls(anchor_hearing_id,anchor_revision,anchor_values_digest,anchor_submission_digest)=4
                AND anchor_revision>=1 AND anchor_revision<=4294967295
                AND octet_length(anchor_values_digest)=32 AND octet_length(anchor_submission_digest)=32));
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_measure_decisions'::regclass
        AND conname='measure_decision_anchor_hearing') THEN
        ALTER TABLE case_measure_decisions ADD CONSTRAINT measure_decision_anchor_hearing
            FOREIGN KEY(anchor_hearing_id,anchor_revision) REFERENCES case_hearing_revisions(hearing_id,revision);
    END IF;
END; $$;
