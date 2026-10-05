DO $$
DECLARE owner_shape TEXT; member_shape TEXT; validity_shape TEXT; matching BIGINT;
BEGIN
    SELECT count(*) INTO matching FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
        WHERE (c.conrelid='case_measure_operations'::regclass AND c.conname='measure_operation_family'
            OR c.conrelid='case_measure_revisions'::regclass AND c.conname IN ('measure_revision_family','measure_revision_validity'))
            AND c.contype='c' AND c.convalidated AND NOT c.connoinherit AND c.conislocal
            AND c.coninhcount=0 AND c.conparentid=0 AND c.connamespace=t.relnamespace
            AND NOT c.condeferrable AND NOT c.condeferred
            AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false';
    IF matching<>3 THEN RAISE EXCEPTION 'measure family constraints are incomplete or altered'; END IF;
    SELECT pg_get_expr(conbin,conrelid) INTO owner_shape FROM pg_constraint
        WHERE conrelid='case_measure_operations'::regclass AND conname='measure_operation_family';
    SELECT pg_get_expr(conbin,conrelid) INTO member_shape FROM pg_constraint
        WHERE conrelid='case_measure_revisions'::regclass AND conname='measure_revision_family';
    SELECT pg_get_expr(conbin,conrelid) INTO validity_shape FROM pg_constraint
        WHERE conrelid='case_measure_revisions'::regclass AND conname='measure_revision_validity';
    IF owner_shape=$old$((family COLLATE "C") = ANY (ARRAY['g1'::text, 'a1'::text]))$old$
        AND member_shape=$old$((family COLLATE "C") = ANY (ARRAY['m1'::text, 'c1'::text]))$old$
        AND validity_shape=$old$((((family COLLATE "C") = 'm1'::text) AND ((validity COLLATE "C") = 'valid'::text)) OR (((family COLLATE "C") = 'c1'::text) AND ((validity COLLATE "C") = ANY (ARRAY['valid'::text, 'entered_in_error'::text]))))$old$ THEN
        ALTER TABLE case_measure_operations DROP CONSTRAINT measure_operation_family;
        ALTER TABLE case_measure_operations ADD CONSTRAINT measure_operation_family
            CHECK(family COLLATE "C" IN ('g1','g2','a1'));
        ALTER TABLE case_measure_revisions DROP CONSTRAINT measure_revision_family;
        ALTER TABLE case_measure_revisions ADD CONSTRAINT measure_revision_family
            CHECK(family COLLATE "C" IN ('m1','m2','c1'));
        ALTER TABLE case_measure_revisions DROP CONSTRAINT measure_revision_validity;
        ALTER TABLE case_measure_revisions ADD CONSTRAINT measure_revision_validity CHECK(
            (family COLLATE "C" IN ('m1','m2') AND validity COLLATE "C"='valid')
            OR (family COLLATE "C"='c1' AND validity COLLATE "C" IN ('valid','entered_in_error')));
    ELSIF owner_shape IS DISTINCT FROM $new$((family COLLATE "C") = ANY (ARRAY['g1'::text, 'g2'::text, 'a1'::text]))$new$
        OR member_shape IS DISTINCT FROM $new$((family COLLATE "C") = ANY (ARRAY['m1'::text, 'm2'::text, 'c1'::text]))$new$
        OR validity_shape IS DISTINCT FROM $new$((((family COLLATE "C") = ANY (ARRAY['m1'::text, 'm2'::text])) AND ((validity COLLATE "C") = 'valid'::text)) OR (((family COLLATE "C") = 'c1'::text) AND ((validity COLLATE "C") = ANY (ARRAY['valid'::text, 'entered_in_error'::text]))))$new$ THEN
        RAISE EXCEPTION 'measure family constraints are not a supported exact shape';
    END IF;
END; $$;
