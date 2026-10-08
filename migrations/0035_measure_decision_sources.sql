DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_measure_revision_source() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE owner RECORD; subject RECORD; supervisor RECORD; bound RECORD;
    source_count BIGINT; source_seconds BIGINT; source_nanos INTEGER; value JSONB; prior RECORD;
BEGIN
    SELECT * INTO owner FROM %1$I.case_measure_decisions WHERE operation_id=NEW.owner_operation;
    IF owner.case_id IS DISTINCT FROM NEW.case_id THEN
        RAISE EXCEPTION 'measure revision requires its exact decision owner' USING ERRCODE='23514';
    END IF;
    IF NEW.revision=1 THEN
        IF NEW.action NOT IN ('impose','substitute_in') OR NOT EXISTS(
            SELECT 1 FROM %1$I.case_measures WHERE id=NEW.measure_id AND case_id=NEW.case_id
                AND initial_revision=1 AND root_operation=NEW.owner_operation) THEN
            RAISE EXCEPTION 'new measure revision lacks its original root' USING ERRCODE='23514';
        END IF;
    ELSE
        SELECT r.*,d.recorded_at_seconds,d.recorded_at_nanoseconds INTO prior
            FROM %1$I.case_measure_revisions r JOIN %1$I.case_measure_decisions d
            ON d.operation_id=r.owner_operation AND d.case_id=r.case_id
            WHERE r.measure_id=NEW.measure_id AND r.revision=NEW.revision-1;
        IF prior.case_id IS DISTINCT FROM NEW.case_id OR prior.family IS DISTINCT FROM 'm1'
            OR NEW.action NOT IN ('confirm','modify','revoke','cease','substitute_out')
            OR prior.action IN ('revoke','cease','substitute_out')
            OR prior.revision IS DISTINCT FROM (SELECT max(revision) FROM %1$I.case_measure_revisions WHERE measure_id=NEW.measure_id)
            OR ROW(owner.recorded_at_seconds,owner.recorded_at_nanoseconds)<ROW(prior.recorded_at_seconds,prior.recorded_at_nanoseconds) THEN
            RAISE EXCEPTION 'measure successor requires its exact current predecessor' USING ERRCODE='23514';
        END IF;
        IF NEW.action='modify' THEN
            IF NEW.subject_id IS DISTINCT FROM prior.subject_id
                OR NEW.subject_revision IS DISTINCT FROM prior.subject_revision
                OR NEW.subject_values_digest IS DISTINCT FROM prior.subject_values_digest
                OR NEW.values_view->'kind' IS DISTINCT FROM prior.values_view->'kind' THEN
                RAISE EXCEPTION 'modification changes exact subject or class' USING ERRCODE='23514';
            END IF;
        ELSIF NEW.values_canonical IS DISTINCT FROM prior.values_canonical
            OR NEW.values_view IS DISTINCT FROM prior.values_view
            OR NEW.values_digest IS DISTINCT FROM prior.values_digest
            OR NEW.subject_id IS DISTINCT FROM prior.subject_id
            OR NEW.subject_revision IS DISTINCT FROM prior.subject_revision
            OR NEW.subject_values_digest IS DISTINCT FROM prior.subject_values_digest
            OR NEW.supervisor_id IS DISTINCT FROM prior.supervisor_id
            OR NEW.supervisor_revision IS DISTINCT FROM prior.supervisor_revision THEN
            RAISE EXCEPTION 'unchanged measure declaration replaces prior values or sources' USING ERRCODE='23514';
        END IF;
    END IF;
    value:=NEW.values_view;
    IF jsonb_typeof(value) IS DISTINCT FROM 'object' OR octet_length(value::text)>32768
        OR NOT(value ?& ARRAY['subject','kind','conditions','validity','supervision'])
        OR value-ARRAY['subject','kind','conditions','validity','supervision']<>'{}'::jsonb
        OR jsonb_typeof(value->'subject') IS DISTINCT FROM 'object'
        OR NEW.subject_id IS DISTINCT FROM (value->'subject'->>'id')::uuid
        OR NEW.subject_revision IS DISTINCT FROM (value->'subject'->>'revision')::bigint
        OR NEW.subject_values_digest IS DISTINCT FROM decode(value->'subject'->>'digest','hex') THEN
        RAISE EXCEPTION 'measure subject selectors differ from values' USING ERRCODE='23514';
    END IF;
    SELECT r.*,s.case_id INTO subject FROM %1$I.case_subject_revisions r
        JOIN %1$I.case_subjects s ON s.id=r.subject_id
        WHERE r.subject_id=NEW.subject_id AND r.revision=NEW.subject_revision;
    IF subject.case_id IS DISTINCT FROM NEW.case_id OR subject.values_digest IS DISTINCT FROM NEW.subject_values_digest THEN
        RAISE EXCEPTION 'measure requires the exact same-case subject' USING ERRCODE='23514';
    END IF;
    source_seconds:=extract(epoch FROM regexp_replace(subject.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
    source_nanos:=rpad(coalesce(substring(subject.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    IF ROW(owner.recorded_at_seconds,owner.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
        RAISE EXCEPTION 'measure capture predates its subject' USING ERRCODE='23514';
    END IF;
    value:=value->'supervision';
    IF jsonb_typeof(value) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION 'measure supervision shape differs' USING ERRCODE='23514';
    END IF;
    IF value->>'kind'='unknown' THEN
        IF NOT(value ?& ARRAY['kind','reason']) OR value-ARRAY['kind','reason']<>'{}'::jsonb
            OR NEW.supervisor_id IS NOT NULL OR NEW.supervisor_revision IS NOT NULL THEN
            RAISE EXCEPTION 'unknown supervision cannot select a participant' USING ERRCODE='23514';
        END IF;
    ELSIF value->>'kind'='known' THEN
        IF NOT(value ?& ARRAY['kind','participant','statement']) OR value-ARRAY['kind','participant','statement']<>'{}'::jsonb
            OR jsonb_typeof(value->'participant') IS DISTINCT FROM 'object'
            OR NEW.supervisor_id IS NULL OR NEW.supervisor_revision IS NULL
            OR NEW.supervisor_id IS DISTINCT FROM (value->'participant'->>'id')::uuid
            OR NEW.supervisor_revision IS DISTINCT FROM (value->'participant'->>'revision')::bigint THEN
            RAISE EXCEPTION 'known supervisor selectors differ from values' USING ERRCODE='23514';
        END IF;
        SELECT count(*) INTO source_count FROM (
            SELECT revision FROM %1$I.case_participant_revisions
                WHERE participant_id=NEW.supervisor_id AND revision=NEW.supervisor_revision
            UNION ALL SELECT revision FROM %1$I.case_participant_typed_revisions
                WHERE participant_id=NEW.supervisor_id AND revision=NEW.supervisor_revision
        ) selected;
        IF source_count<>1 THEN
            RAISE EXCEPTION 'supervisor requires exactly one historical source' USING ERRCODE='23514';
        END IF;
        SELECT r.*,p.case_id INTO supervisor FROM (
            SELECT participant_id,changed_at,NULL::uuid AS subject_id,NULL::bigint AS subject_revision,NULL::bytea AS subject_digest
                FROM %1$I.case_participant_revisions WHERE participant_id=NEW.supervisor_id AND revision=NEW.supervisor_revision
            UNION ALL SELECT participant_id,changed_at,subject_id,subject_revision,decode(values_view->'subject'->>'digest','hex')
                FROM %1$I.case_participant_typed_revisions WHERE participant_id=NEW.supervisor_id AND revision=NEW.supervisor_revision
        ) r JOIN %1$I.case_participants p ON p.id=r.participant_id;
        IF supervisor.case_id IS DISTINCT FROM NEW.case_id THEN
            RAISE EXCEPTION 'measure supervisor belongs to another case' USING ERRCODE='23514';
        END IF;
        source_seconds:=extract(epoch FROM regexp_replace(supervisor.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
        source_nanos:=rpad(coalesce(substring(supervisor.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
        IF ROW(owner.recorded_at_seconds,owner.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
            RAISE EXCEPTION 'measure capture predates its supervisor' USING ERRCODE='23514';
        END IF;
        IF supervisor.subject_id IS NOT NULL THEN
            SELECT r.*,s.case_id INTO bound FROM %1$I.case_subject_revisions r
                JOIN %1$I.case_subjects s ON s.id=r.subject_id
                WHERE r.subject_id=supervisor.subject_id AND r.revision=supervisor.subject_revision;
            IF bound.case_id IS DISTINCT FROM NEW.case_id OR bound.values_digest IS DISTINCT FROM supervisor.subject_digest THEN
                RAISE EXCEPTION 'typed supervisor requires its exact historical subject' USING ERRCODE='23514';
            END IF;
            source_seconds:=extract(epoch FROM regexp_replace(bound.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
            source_nanos:=rpad(coalesce(substring(bound.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
            IF ROW(owner.recorded_at_seconds,owner.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos) THEN
                RAISE EXCEPTION 'measure capture predates the supervisor subject' USING ERRCODE='23514';
            END IF;
        END IF;
    ELSE
        RAISE EXCEPTION 'unknown supervision variant' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION enforce_measure_revision_source() FROM PUBLIC;
