-- Resource hearing alert projections retain their exact case and resource parent.
ALTER TABLE alert_subject_state ADD COLUMN IF NOT EXISTS resource_id UUID;
ALTER TABLE alert_schedule ADD COLUMN IF NOT EXISTS resource_id UUID;
ALTER TABLE alert_notifications ADD COLUMN IF NOT EXISTS resource_id UUID;

DO $$ DECLARE tab TEXT; constraint_name TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['alert_subject_state','alert_scan_cursor'] LOOP
        constraint_name := tab || '_kind_check';
        IF EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=tab::regclass
            AND c.conname=constraint_name AND c.contype='c'
            AND pg_get_expr(c.conbin,c.conrelid)='(kind = ANY (ARRAY[0, 1]))') THEN
            EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',tab,constraint_name);
            EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK(kind IN (0,1,2))',
                tab,constraint_name);
        END IF;
    END LOOP;
    SELECT c.conname INTO constraint_name FROM pg_constraint c
        WHERE c.conrelid='alert_scan_cursor'::regclass AND c.contype='c'
            AND pg_get_expr(c.conbin,c.conrelid)=
                '(((active_kind IS NULL) AND (active_id IS NULL) AND (after_recipient IS NULL)) OR ((active_kind = ANY (ARRAY[0, 1])) AND (active_id IS NOT NULL)))';
    IF constraint_name IS NOT NULL THEN
        EXECUTE format('ALTER TABLE alert_scan_cursor DROP CONSTRAINT %I',constraint_name);
        EXECUTE format('ALTER TABLE alert_scan_cursor ADD CONSTRAINT %I CHECK(
            (active_kind IS NULL AND active_id IS NULL AND after_recipient IS NULL)
            OR (active_kind IN (0,1,2) AND active_id IS NOT NULL))',constraint_name);
    END IF;
    FOREACH tab IN ARRAY ARRAY['alert_subject_state','alert_schedule','alert_notifications'] LOOP
        constraint_name := tab || '_resource_shape';
        IF NOT EXISTS(SELECT 1 FROM pg_constraint
            WHERE conrelid=tab::regclass AND conname=constraint_name) THEN
            EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK(
                (kind IN (0,1) AND resource_id IS NULL)
                OR (kind=2 AND resource_id IS NOT NULL))',tab,constraint_name);
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint
        WHERE conrelid='alert_subject_state'::regclass AND conname='alert_subject_resource_scope') THEN
        ALTER TABLE alert_subject_state ADD CONSTRAINT alert_subject_resource_scope
            UNIQUE(kind,id,case_id,resource_id);
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint
        WHERE conrelid='alert_subject_state'::regclass AND conname='alert_subject_resource_hearing') THEN
        ALTER TABLE alert_subject_state ADD CONSTRAINT alert_subject_resource_hearing
            FOREIGN KEY(id,case_id,resource_id)
                REFERENCES case_resource_hearings(id,case_id,resource_id);
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint
        WHERE conrelid='alert_schedule'::regclass AND conname='alert_schedule_resource_subject') THEN
        ALTER TABLE alert_schedule ADD CONSTRAINT alert_schedule_resource_subject
            FOREIGN KEY(kind,subject_id,case_id,resource_id)
                REFERENCES alert_subject_state(kind,id,case_id,resource_id);
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint
        WHERE conrelid='alert_notifications'::regclass AND conname='alert_notifications_resource_subject') THEN
        ALTER TABLE alert_notifications ADD CONSTRAINT alert_notifications_resource_subject
            FOREIGN KEY(kind,subject_id,case_id,resource_id)
                REFERENCES alert_subject_state(kind,id,case_id,resource_id);
    END IF;
END; $$;
