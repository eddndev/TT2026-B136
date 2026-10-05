DO $$ BEGIN
    IF EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_measure_revisions'::regclass
        AND conname='measure_revision_initial') THEN
        ALTER TABLE case_measure_revisions DROP CONSTRAINT measure_revision_initial;
        ALTER TABLE case_measure_revisions ADD CONSTRAINT measure_revision_range CHECK(revision BETWEEN 1 AND 4294967295);
        ALTER TABLE case_measure_revisions DROP CONSTRAINT measure_revision_action;
        ALTER TABLE case_measure_revisions ADD CONSTRAINT measure_revision_action CHECK(action COLLATE "C" IN
            ('impose','confirm','modify','revoke','cease','substitute_out','substitute_in'));
    END IF;
END; $$;
