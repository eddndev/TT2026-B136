DO $$
DECLARE present BIGINT; action_shape TEXT; correction_shape TEXT; matching BIGINT;
BEGIN
    SELECT count(*) INTO present FROM pg_attribute WHERE attrelid='case_measure_administrations'::regclass
        AND attname IN ('replacement_measure_id','replacement_subject_id','replacement_subject_revision',
            'replacement_subject_values_digest') AND NOT attisdropped;
    IF present=0 THEN
        SELECT count(*) INTO matching FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid='case_measure_administrations'::regclass
                AND c.conname IN ('measure_administration_action','measure_administration_correction_shape')
                AND c.contype='c' AND c.convalidated AND NOT c.connoinherit AND c.conislocal
                AND c.coninhcount=0 AND c.conparentid=0 AND c.connamespace=t.relnamespace
                AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false';
        SELECT pg_get_expr(conbin,conrelid) INTO action_shape FROM pg_constraint
            WHERE conrelid='case_measure_administrations'::regclass AND conname='measure_administration_action';
        SELECT pg_get_expr(conbin,conrelid) INTO correction_shape FROM pg_constraint
            WHERE conrelid='case_measure_administrations'::regclass AND conname='measure_administration_correction_shape';
        IF matching<>2 OR action_shape IS DISTINCT FROM $old$((action COLLATE "C") = ANY (ARRAY['correct'::text, 'entered_in_error'::text]))$old$
            OR correction_shape IS DISTINCT FROM $old$((((action COLLATE "C") = 'correct'::text) AND (num_nonnulls(correction_canonical, correction_view, correction_digest) = 3)) OR (((action COLLATE "C") = 'entered_in_error'::text) AND (num_nonnulls(correction_canonical, correction_view, correction_digest) = 0)))$old$ THEN
            RAISE EXCEPTION 'administrative action constraints are incomplete or altered';
        END IF;
        ALTER TABLE case_measure_administrations
            ADD COLUMN replacement_measure_id UUID,
            ADD COLUMN replacement_subject_id UUID,
            ADD COLUMN replacement_subject_revision BIGINT,
            ADD COLUMN replacement_subject_values_digest BYTEA;
        ALTER TABLE case_measure_administrations DROP CONSTRAINT measure_administration_action;
        ALTER TABLE case_measure_administrations ADD CONSTRAINT measure_administration_action
            CHECK(action COLLATE "C" IN ('correct','entered_in_error','replace_entered_in_error'));
        ALTER TABLE case_measure_administrations DROP CONSTRAINT measure_administration_correction_shape;
        ALTER TABLE case_measure_administrations ADD CONSTRAINT measure_administration_correction_shape CHECK(
            (action COLLATE "C"='correct' AND num_nonnulls(correction_canonical,correction_view,correction_digest)=3)
            OR (action COLLATE "C" IN ('entered_in_error','replace_entered_in_error')
                AND num_nonnulls(correction_canonical,correction_view,correction_digest)=0));
        ALTER TABLE case_measure_administrations ADD CONSTRAINT measure_administration_replacement_shape CHECK(
            (action COLLATE "C" IN ('correct','entered_in_error') AND num_nonnulls(replacement_measure_id,
                replacement_subject_id,replacement_subject_revision,replacement_subject_values_digest)=0)
            OR (action COLLATE "C"='replace_entered_in_error' AND num_nonnulls(replacement_measure_id,
                replacement_subject_id,replacement_subject_revision,replacement_subject_values_digest)=4
                AND replacement_measure_id<>target_measure_id
                AND replacement_subject_revision BETWEEN 1 AND 4294967295
                AND octet_length(replacement_subject_values_digest)=32));
        ALTER TABLE case_measure_administrations ADD CONSTRAINT measure_administration_replacement_subject
            FOREIGN KEY(replacement_subject_id,replacement_subject_revision)
            REFERENCES case_subject_revisions(subject_id,revision);
        ALTER TABLE case_measure_administrations ADD CONSTRAINT measure_administration_replacement_root
            FOREIGN KEY(replacement_measure_id,case_id) REFERENCES case_measures(id,case_id)
            DEFERRABLE INITIALLY DEFERRED;
    ELSIF present<>4 THEN
        RAISE EXCEPTION 'administrative replacement selectors are incomplete';
    END IF;
END; $$;
