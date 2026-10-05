-- Add precautionary appointments without changing existing alert identities or policies.
DO $$ DECLARE tab TEXT; constraint_name TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['alert_subject_state','alert_scan_cursor'] LOOP
        constraint_name := tab || '_kind_check';
        IF EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=tab::regclass
            AND c.conname=constraint_name AND c.contype='c'
            AND pg_get_expr(c.conbin,c.conrelid)='(kind = ANY (ARRAY[0, 1, 2]))') THEN
            EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',tab,constraint_name);
            EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK(kind IN (0,1,2,3))',
                tab,constraint_name);
        END IF;
    END LOOP;
    SELECT c.conname INTO constraint_name FROM pg_constraint c
        WHERE c.conrelid='alert_scan_cursor'::regclass AND c.contype='c'
            AND pg_get_expr(c.conbin,c.conrelid)=
                '(((active_kind IS NULL) AND (active_id IS NULL) AND (after_recipient IS NULL)) OR ((active_kind = ANY (ARRAY[0, 1, 2])) AND (active_id IS NOT NULL)))';
    IF constraint_name IS NOT NULL THEN
        EXECUTE format('ALTER TABLE alert_scan_cursor DROP CONSTRAINT %I',constraint_name);
        EXECUTE format('ALTER TABLE alert_scan_cursor ADD CONSTRAINT %I CHECK(
            (active_kind IS NULL AND active_id IS NULL AND after_recipient IS NULL)
            OR (active_kind IN (0,1,2,3) AND active_id IS NOT NULL))',constraint_name);
    END IF;
    FOREACH tab IN ARRAY ARRAY['alert_subject_state','alert_schedule','alert_notifications'] LOOP
        constraint_name := tab || '_resource_shape';
        IF EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=tab::regclass
            AND c.conname=constraint_name AND c.contype='c'
            AND pg_get_expr(c.conbin,c.conrelid)=
                '(((kind = ANY (ARRAY[0, 1])) AND (resource_id IS NULL)) OR ((kind = 2) AND (resource_id IS NOT NULL)))') THEN
            EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',tab,constraint_name);
            EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK(
                (kind IN (0,1,3) AND resource_id IS NULL)
                OR (kind=2 AND resource_id IS NOT NULL))',tab,constraint_name);
        END IF;
    END LOOP;
END; $$;
