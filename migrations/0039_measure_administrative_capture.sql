DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_measure_administration_capture() RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE actor RECORD; administration RECORD; current_stage RECORD; stage_origin RECORD; owner RECORD;
    prior RECORD; prior_owner RECORD; prior_administration RECORD; prior_stage RECORD; lineage RECORD;
    administrative RECORD; judicial RECORD; audit_row RECORD; source_count BIGINT; depth INTEGER:=0;
    source_seconds BIGINT; source_nanos INTEGER; admin_seconds BIGINT; admin_nanos INTEGER;
    marker TEXT; recorded TEXT; selected JSONB; temporal JSONB; keys TEXT[]; component TEXT; expected_values JSONB;
BEGIN
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE
        OR actor.email COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C"
        OR actor.role COLLATE "C" IS DISTINCT FROM NEW.recorded_by_role COLLATE "C"
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by FOR SHARE))) THEN
        RAISE EXCEPTION 'administrative measure actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS DISTINCT FROM NEW.observed_administration_revision
        OR administration.administrative_status IS DISTINCT FROM 'active' OR administration.nuc IS NULL THEN
        RAISE EXCEPTION 'administrative measure requires current active complete administration' USING ERRCODE='23514';
    END IF;
    SELECT stages.*,count(*) OVER() AS source_count INTO current_stage FROM (
        SELECT revision,administration_revision,recorded_at_seconds,recorded_at_nanoseconds
            FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id
        UNION ALL SELECT i.stage_revision,i.administration_revision,
            extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
            rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
            FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                ON a.case_id=i.case_id AND a.revision=i.administration_revision WHERE i.case_id=NEW.case_id
    ) stages ORDER BY revision DESC LIMIT 1;
    SELECT count(*) INTO source_count FROM (
        SELECT revision FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id AND revision=NEW.observed_stage_revision
        UNION ALL SELECT stage_revision FROM %1$I.case_initial_stage_registrations
            WHERE case_id=NEW.case_id AND stage_revision=NEW.observed_stage_revision
    ) stages;
    IF source_count<>1 OR current_stage.revision IS DISTINCT FROM NEW.observed_stage_revision
        OR current_stage.administration_revision>NEW.observed_administration_revision THEN
        RAISE EXCEPTION 'administrative measure requires the exact current stage' USING ERRCODE='23514';
    END IF;
    SELECT * INTO stage_origin FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id AND revision=current_stage.administration_revision;
    source_seconds:=extract(epoch FROM regexp_replace(stage_origin.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
    source_nanos:=rpad(coalesce(substring(stage_origin.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    admin_seconds:=extract(epoch FROM regexp_replace(administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
    admin_nanos:=rpad(coalesce(substring(administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    IF stage_origin.administrative_status IS DISTINCT FROM 'active' OR stage_origin.nuc IS NULL
        OR ROW(admin_seconds,admin_nanos)<ROW(source_seconds,source_nanos)
        OR ROW(current_stage.recorded_at_seconds,current_stage.recorded_at_nanoseconds)<ROW(source_seconds,source_nanos)
        OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)<ROW(admin_seconds,admin_nanos)
        OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)
            <ROW(current_stage.recorded_at_seconds,current_stage.recorded_at_nanoseconds) THEN
        RAISE EXCEPTION 'administrative measure predates observed context sources' USING ERRCODE='23514';
    END IF;
    SELECT * INTO owner FROM %1$I.case_measure_operations WHERE operation_id=NEW.operation_id;
    IF owner.case_id IS DISTINCT FROM NEW.case_id OR owner.family IS DISTINCT FROM 'a1'
        OR owner.owner_digest IS DISTINCT FROM NEW.capture_digest
        OR EXISTS(SELECT 1 FROM %1$I.case_measure_decisions WHERE operation_id=NEW.operation_id) THEN
        RAISE EXCEPTION 'administrative measure requires its exact exclusive owner' USING ERRCODE='23514';
    END IF;
    SELECT * INTO prior FROM %1$I.case_measure_revisions
        WHERE measure_id=NEW.target_measure_id AND revision=NEW.target_revision;
    IF prior.case_id IS DISTINCT FROM NEW.case_id OR prior.family NOT IN ('m1','c1')
        OR prior.capture_digest IS DISTINCT FROM NEW.target_capture_digest OR prior.validity IS DISTINCT FROM 'valid'
        OR prior.revision NOT BETWEEN 1 AND 4294967294
        OR prior.revision IS DISTINCT FROM (SELECT max(revision) FROM %1$I.case_measure_revisions WHERE measure_id=NEW.target_measure_id)
        OR NOT EXISTS(SELECT 1 FROM %1$I.case_measures root JOIN %1$I.case_measure_revisions first
            ON first.measure_id=root.id AND first.case_id=root.case_id AND first.revision=1
                AND first.owner_operation=root.root_operation AND first.family='m1' AND first.validity='valid'
            JOIN %1$I.case_measure_operations origin ON origin.operation_id=root.root_operation
                AND origin.case_id=root.case_id AND origin.family='g1'
            WHERE root.id=NEW.target_measure_id AND root.case_id=NEW.case_id AND root.initial_revision=1
                AND first.action IN ('impose','substitute_in')) THEN
        RAISE EXCEPTION 'administrative measure requires its exact valid current target and judicial root' USING ERRCODE='23514';
    END IF;
    SELECT sources.*,count(*) OVER() AS source_count INTO prior_owner FROM (
        SELECT 'm1'::text AS member_family,d.recorded_at_seconds,d.recorded_at_nanoseconds,
            d.observed_administration_revision,d.observed_stage_revision
            FROM %1$I.case_measure_decisions d JOIN %1$I.case_measure_operations o
                ON o.operation_id=d.operation_id AND o.case_id=d.case_id AND o.owner_digest=d.group_digest AND o.family='g1'
            WHERE d.operation_id=prior.owner_operation AND d.case_id=NEW.case_id
        UNION ALL SELECT 'c1',a.recorded_at_seconds,a.recorded_at_nanoseconds,
            a.observed_administration_revision,a.observed_stage_revision
            FROM %1$I.case_measure_administrations a JOIN %1$I.case_measure_operations o
                ON o.operation_id=a.operation_id AND o.case_id=a.case_id AND o.owner_digest=a.capture_digest AND o.family='a1'
            WHERE a.operation_id=prior.owner_operation AND a.case_id=NEW.case_id
    ) sources;
    IF prior_owner.source_count IS DISTINCT FROM 1::bigint OR prior_owner.member_family IS DISTINCT FROM prior.family
        OR prior_owner.observed_administration_revision>NEW.observed_administration_revision
        OR prior_owner.observed_stage_revision>NEW.observed_stage_revision
        OR ROW(NEW.recorded_at_seconds,NEW.recorded_at_nanoseconds)
            <ROW(prior_owner.recorded_at_seconds,prior_owner.recorded_at_nanoseconds) THEN
        RAISE EXCEPTION 'administrative measure target owner or chronology differs' USING ERRCODE='23514';
    END IF;
    SELECT * INTO prior_administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id AND revision=prior_owner.observed_administration_revision;
    source_seconds:=extract(epoch FROM regexp_replace(prior_administration.changed_at,'\.[0-9]+','')::timestamptz)::bigint;
    source_nanos:=rpad(coalesce(substring(prior_administration.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer;
    IF prior_administration.administrative_status IS DISTINCT FROM 'active' OR prior_administration.nuc IS NULL
        OR ROW(admin_seconds,admin_nanos)<ROW(source_seconds,source_nanos) THEN
        RAISE EXCEPTION 'administrative measure observed administration regressed' USING ERRCODE='23514';
    END IF;
    SELECT stages.*,count(*) OVER() AS source_count INTO prior_stage FROM (
        SELECT revision,recorded_at_seconds,recorded_at_nanoseconds FROM %1$I.case_stage_revisions
            WHERE case_id=NEW.case_id AND revision=prior_owner.observed_stage_revision
        UNION ALL SELECT i.stage_revision,
            extract(epoch FROM regexp_replace(a.changed_at,'\.[0-9]+','')::timestamptz)::bigint,
            rpad(coalesce(substring(a.changed_at FROM '\.([0-9]+)'),'0'),9,'0')::integer
            FROM %1$I.case_initial_stage_registrations i JOIN %1$I.case_administration_revisions a
                ON a.case_id=i.case_id AND a.revision=i.administration_revision
            WHERE i.case_id=NEW.case_id AND i.stage_revision=prior_owner.observed_stage_revision
    ) stages;
    IF prior_stage.source_count IS DISTINCT FROM 1::bigint
        OR ROW(current_stage.recorded_at_seconds,current_stage.recorded_at_nanoseconds)
            <ROW(prior_stage.recorded_at_seconds,prior_stage.recorded_at_nanoseconds) THEN
        RAISE EXCEPTION 'administrative measure observed stage regressed' USING ERRCODE='23514';
    END IF;
    lineage:=prior;
    LOOP
        depth:=depth+1;
        IF depth>255 THEN
            RAISE EXCEPTION 'administrative measure predecessor lineage exceeds bound' USING ERRCODE='23514';
        END IF;
        IF lineage.family='m1' THEN
            SELECT d.* INTO judicial FROM %1$I.case_measure_decisions d JOIN %1$I.case_measure_operations o
                ON o.operation_id=d.operation_id AND o.case_id=d.case_id AND o.owner_digest=d.group_digest AND o.family='g1'
                WHERE d.operation_id=lineage.owner_operation AND d.case_id=NEW.case_id;
            IF judicial.operation_id IS NULL THEN
                RAISE EXCEPTION 'administrative measure last judicial owner is absent' USING ERRCODE='23514';
            END IF;
            EXIT;
        ELSIF lineage.family='c1' THEN
            SELECT a.* INTO administrative FROM %1$I.case_measure_administrations a JOIN %1$I.case_measure_operations o
                ON o.operation_id=a.operation_id AND o.case_id=a.case_id AND o.owner_digest=a.capture_digest AND o.family='a1'
                WHERE a.operation_id=lineage.owner_operation AND a.case_id=NEW.case_id;
            IF administrative.operation_id IS NULL OR administrative.target_measure_id IS DISTINCT FROM NEW.target_measure_id
                OR administrative.target_revision IS DISTINCT FROM lineage.revision-1 THEN
                RAISE EXCEPTION 'administrative measure predecessor lineage differs' USING ERRCODE='23514';
            END IF;
            SELECT * INTO lineage FROM %1$I.case_measure_revisions
                WHERE measure_id=administrative.target_measure_id AND revision=administrative.target_revision
                    AND case_id=NEW.case_id AND capture_digest=administrative.target_capture_digest;
            IF lineage.measure_id IS NULL OR lineage.validity IS DISTINCT FROM 'valid' THEN
                RAISE EXCEPTION 'administrative measure predecessor lineage is absent or invalid' USING ERRCODE='23514';
            END IF;
        ELSE
            RAISE EXCEPTION 'unsupported administrative predecessor family' USING ERRCODE='23514';
        END IF;
    END LOOP;
    IF NEW.support_format IS DISTINCT FROM judicial.support_format OR NEW.support_policy IS DISTINCT FROM judicial.support_policy
        OR NOT EXISTS(SELECT 1 FROM %1$I.documents WHERE case_id=NEW.case_id
            AND id=(judicial.values_view->'support'->>'document_id')::uuid
            AND version=(judicial.values_view->'support'->>'version')::bigint
            AND digest=decode(judicial.values_view->'support'->>'digest','hex')) THEN
        RAISE EXCEPTION 'administrative measure replaces actual last judicial support' USING ERRCODE='23514';
    END IF;
    selected:=jsonb_build_object('id',NEW.target_measure_id::text,'revision',NEW.target_revision,'digest',encode(NEW.target_capture_digest,'hex'));
    IF EXISTS(SELECT 1 FROM %1$I.case_measure_administrations a WHERE a.case_id=NEW.case_id
            AND a.target_measure_id=NEW.target_measure_id AND a.target_revision=NEW.target_revision
            AND a.target_capture_digest=NEW.target_capture_digest)
        OR EXISTS(SELECT 1 FROM %1$I.case_measure_decisions d, LATERAL jsonb_array_elements(
            CASE WHEN d.outcome_view->>'kind'='changes' THEN d.outcome_view->'effects' ELSE '[]'::jsonb END) e
            WHERE d.case_id=NEW.case_id AND (e->'previous'=selected
                OR (e->>'action'='substitute' AND e->'predecessors' @> jsonb_build_array(selected))))
        OR EXISTS(SELECT 1 FROM %1$I.case_precautionary_hearing_revisions h WHERE h.case_id=NEW.case_id
            AND h.action IN ('schedule','replace') AND h.values_view->>'purpose'='review'
            AND h.values_view->'review_targets' @> jsonb_build_array(selected)) THEN
        RAISE EXCEPTION 'administrative measure target has an existing declared dependant' USING ERRCODE='23514';
    END IF;
    IF NEW.action='correct' THEN
        IF num_nonnulls(NEW.correction_canonical,NEW.correction_view,NEW.correction_digest)<>3
            OR jsonb_typeof(NEW.correction_view) IS DISTINCT FROM 'object'
            OR octet_length(NEW.correction_view::text)>32768
            OR NOT(NEW.correction_view ?& ARRAY['conditions','validity','supervision_text'])
            OR NEW.correction_view-ARRAY['conditions','validity','supervision_text']<>'{}'::jsonb
            OR jsonb_typeof(NEW.correction_view->'validity') IS DISTINCT FROM 'object' THEN
            RAISE EXCEPTION 'correction fields exceed exact shape' USING ERRCODE='23514';
        END IF;
        selected:=NEW.correction_view->'validity';
        IF NOT(selected ?& ARRAY['start','statement','end']) OR selected-ARRAY['start','statement','end']<>'{}'::jsonb
            OR jsonb_typeof(NEW.correction_view->'conditions') IS DISTINCT FROM 'string'
            OR jsonb_typeof(NEW.correction_view->'supervision_text') IS DISTINCT FROM 'string'
            OR jsonb_typeof(selected->'statement') IS DISTINCT FROM 'string'
            OR %1$I.case_administration_text_valid(NEW.correction_view->>'conditions',1000,TRUE) IS DISTINCT FROM TRUE
            OR %1$I.case_administration_text_valid(NEW.correction_view->>'supervision_text',1000,TRUE) IS DISTINCT FROM TRUE
            OR %1$I.case_administration_text_valid(selected->>'statement',1000,TRUE) IS DISTINCT FROM TRUE
            OR (prior.values_view->'validity'->'end'='null'::jsonb) IS DISTINCT FROM (selected->'end'='null'::jsonb) THEN
            RAISE EXCEPTION 'correction changes validity shape or contains invalid text' USING ERRCODE='23514';
        END IF;
        FOR temporal IN SELECT selected->'start' UNION ALL SELECT selected->'end' WHERE selected->'end'<>'null'::jsonb LOOP
            IF jsonb_typeof(temporal) IS DISTINCT FROM 'object' THEN
                RAISE EXCEPTION 'correction time must be an object' USING ERRCODE='23514';
            END IF;
            keys:=CASE temporal->>'precision'
                WHEN 'unknown' THEN ARRAY['precision','reason']
                WHEN 'date' THEN ARRAY['precision','year','month','day','offset_seconds']
                WHEN 'minute' THEN ARRAY['precision','year','month','day','hour','minute','offset_seconds']
                WHEN 'second' THEN ARRAY['precision','year','month','day','hour','minute','second','offset_seconds'] END;
            IF keys IS NULL OR NOT(temporal ?& keys) OR temporal-keys<>'{}'::jsonb THEN
                RAISE EXCEPTION 'correction time fields differ' USING ERRCODE='23514';
            END IF;
            IF temporal->>'precision'='unknown' THEN
                IF jsonb_typeof(temporal->'reason') IS DISTINCT FROM 'string'
                    OR %1$I.case_administration_text_valid(temporal->>'reason',1000,TRUE) IS DISTINCT FROM TRUE THEN
                    RAISE EXCEPTION 'unknown correction time requires a reason' USING ERRCODE='23514';
                END IF;
            ELSE
                FOREACH component IN ARRAY keys LOOP
                    IF component='precision' OR (component='offset_seconds' AND temporal->component='null'::jsonb) THEN CONTINUE; END IF;
                    IF jsonb_typeof(temporal->component) IS DISTINCT FROM 'number'
                        OR coalesce(temporal->>component,'') COLLATE "C" !~ '^-?[0-9]{1,9}$' THEN
                        RAISE EXCEPTION 'correction time requires bounded integer components' USING ERRCODE='23514';
                    END IF;
                END LOOP;
                IF (temporal->>'year')::integer NOT BETWEEN 1 AND 9999
                    OR (temporal->>'month')::integer NOT BETWEEN 1 AND 12
                    OR (temporal->>'day')::integer NOT BETWEEN 1 AND 31
                    OR (temporal->>'hour')::integer NOT BETWEEN 0 AND 23
                    OR (temporal->>'minute')::integer NOT BETWEEN 0 AND 59
                    OR (temporal->>'second')::integer NOT BETWEEN 0 AND 59
                    OR (temporal->>'offset_seconds')::integer NOT BETWEEN -86399 AND 86399 THEN
                    RAISE EXCEPTION 'correction time component exceeds range' USING ERRCODE='23514';
                END IF;
                PERFORM make_date((temporal->>'year')::integer,(temporal->>'month')::integer,(temporal->>'day')::integer);
            END IF;
        END LOOP;
        IF prior.values_view->'supervision'->>'kind' NOT IN ('known','unknown') THEN
            RAISE EXCEPTION 'correction supervision variant is unsupported' USING ERRCODE='23514';
        END IF;
        expected_values:=prior.values_view||jsonb_build_object('conditions',NEW.correction_view->'conditions',
            'validity',NEW.correction_view->'validity','supervision',(prior.values_view->'supervision')||jsonb_build_object(
                CASE prior.values_view->'supervision'->>'kind' WHEN 'known' THEN 'statement' ELSE 'reason' END,
                NEW.correction_view->'supervision_text'));
        IF expected_values IS NOT DISTINCT FROM prior.values_view THEN
            RAISE EXCEPTION 'correction does not change the effective declaration' USING ERRCODE='23514';
        END IF;
    ELSIF NEW.action='entered_in_error' THEN
        IF num_nonnulls(NEW.correction_canonical,NEW.correction_view,NEW.correction_digest)<>0 THEN
            RAISE EXCEPTION 'entered-in-error action cannot contain correction values' USING ERRCODE='23514';
        END IF;
    ELSE
        RAISE EXCEPTION 'unsupported administrative measure action' USING ERRCODE='23514';
    END IF;
    recorded:=to_char(to_timestamp(NEW.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
        ||CASE WHEN NEW.recorded_at_nanoseconds=0 THEN 'Z'
            ELSE '.'||rtrim(lpad(NEW.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END;
    marker:='ma1:case:'||NEW.case_id::text||':operation:'||NEW.operation_id::text
        ||':measure:'||NEW.target_measure_id::text||':revision:'||(NEW.target_revision+1)::text
        ||':submission:'||encode(NEW.submission_digest,'hex')||':review:'||encode(NEW.review_digest,'hex')
        ||':capture:'||encode(NEW.capture_digest,'hex');
    SELECT count(*) INTO source_count FROM (SELECT 1 FROM %1$I.audit_events e
        WHERE e.resource COLLATE "C"=marker COLLATE "C" AND e.action='measure_administrative.recorded' LIMIT 2) events;
    SELECT e.* INTO audit_row FROM %1$I.audit_events e WHERE e.sequence=owner.audit_sequence
        AND octet_length(e.timestamp) BETWEEN 1 AND 64 AND octet_length(e.actor) BETWEEN 1 AND 1280
        AND octet_length(e.action) BETWEEN 1 AND 64 AND octet_length(e.resource) BETWEEN 1 AND 512 AND octet_length(e.chain)=32;
    IF source_count<>1 OR audit_row.sequence IS NULL
        OR audit_row.actor COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C"
        OR audit_row.action COLLATE "C" IS DISTINCT FROM 'measure_administrative.recorded' COLLATE "C"
        OR audit_row.resource COLLATE "C" IS DISTINCT FROM marker COLLATE "C"
        OR audit_row.timestamp COLLATE "C" IS DISTINCT FROM recorded COLLATE "C" THEN
        RAISE EXCEPTION 'administrative measure original audit marker differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION enforce_measure_administration_capture() FROM PUBLIC;
