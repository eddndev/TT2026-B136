-- Compare canonical projections with immediate historical sources, never their heads.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_procedural_fact_sources(scope UUID,family TEXT,value JSONB,sources JSONB)
RETURNS VOID LANGUAGE plpgsql VOLATILE SET search_path=pg_catalog AS $$
DECLARE selected JSONB; expected JSONB; actual JSONB; field TEXT; persons JSONB:='[]';
    provenances JSONB; support_refs JSONB; parent_row RECORD; manual_row RECORD; typed_row RECORD;
    subject_row RECORD; hearing_row RECORD; source_count BIGINT; values_json JSONB;
    subject_json JSONB; temporal JSONB; local_time TIMESTAMP; agreement JSONB;
BEGIN
    -- Shape and scalar policies are established by the canonical byte readers.
    IF scope IS NULL OR family IS NULL OR family NOT IN ('resolution','notification')
        OR jsonb_typeof(value) IS DISTINCT FROM 'object' OR jsonb_typeof(sources) IS DISTINCT FROM 'object'
        OR NOT(sources?'resolution') OR NOT EXISTS(SELECT 1 FROM %1$I.cases WHERE id=scope) THEN
        RAISE EXCEPTION 'invalid procedural fact source validation input' USING ERRCODE='23514'; END IF;
    FOREACH field IN ARRAY ARRAY['participants','hearing_results','direct_supports'] LOOP
        IF jsonb_typeof(sources->field) IS DISTINCT FROM 'array' THEN
            RAISE EXCEPTION 'procedural fact source list is absent' USING ERRCODE='23514'; END IF;
        IF jsonb_array_length(sources->field)>(CASE field WHEN 'participants' THEN 4 ELSE 2 END) THEN
            RAISE EXCEPTION 'procedural fact source list exceeds limit' USING ERRCODE='23514'; END IF;
    END LOOP;
    selected:=sources->'resolution';
    IF family='resolution' THEN
        IF selected IS DISTINCT FROM 'null'::jsonb THEN
            RAISE EXCEPTION 'resolution must not select a parent' USING ERRCODE='23514'; END IF;
    ELSE
        IF jsonb_typeof(selected) IS DISTINCT FROM 'object'
            OR jsonb_build_object('id',selected->'id','revision',selected->'revision')
                IS DISTINCT FROM value->'resolution' THEN
            RAISE EXCEPTION 'notification must select its exact parent revision' USING ERRCODE='23514'; END IF;
        SELECT * INTO parent_row FROM %1$I.case_procedural_fact_revisions r
            WHERE r.family='resolution' AND r.case_id=scope AND r.id=(selected->>'id')::uuid
                AND r.revision=(selected->>'revision')::bigint;
        IF NOT FOUND OR parent_row.values_digest IS DISTINCT FROM sha256(parent_row.values_canonical)
            OR parent_row.submission_digest IS DISTINCT FROM sha256(parent_row.submission_canonical)
            OR parent_row.sources_digest IS DISTINCT FROM sha256(parent_row.sources_canonical) THEN
            RAISE EXCEPTION 'parent resolution is absent or inconsistent' USING ERRCODE='23514'; END IF;
        values_json:=%1$I.procedural_fact_values('resolution',parent_row.values_canonical);
        IF values_json IS DISTINCT FROM parent_row.values_view THEN
            RAISE EXCEPTION 'parent resolution projection differs from canonical content' USING ERRCODE='23514'; END IF;
        temporal:=values_json->'issued_at';
        IF temporal->>'precision'<>'unknown' THEN
            temporal:=(temporal-ARRAY['year','month','day'])||jsonb_build_object('date',
                to_char(make_date((temporal->>'year')::integer,(temporal->>'month')::integer,
                    (temporal->>'day')::integer),'YYYY-MM-DD'));
        END IF;
        expected:=jsonb_build_object('case',scope,'id',parent_row.id,'revision',parent_row.revision,
            'values_digest',encode(parent_row.values_digest,'hex'),'submission_digest',encode(parent_row.submission_digest,'hex'),
            'status',parent_row.status,'issued_at',temporal,'summary',values_json->'summary');
        FOREACH field IN ARRAY ARRAY['class','issuer'] LOOP
            IF values_json#>>ARRAY[field,'kind']='known' THEN actual:=jsonb_build_object('known',values_json#>ARRAY[field,'value']);
            ELSE actual:=jsonb_build_object('unknown',values_json#>ARRAY[field,'reason']); END IF;
            expected:=expected||jsonb_build_object(field,actual);
        END LOOP;
        IF expected IS DISTINCT FROM selected THEN
            RAISE EXCEPTION 'parent resolution snapshot differs from exact history' USING ERRCODE='23514'; END IF;
        FOREACH field IN ARRAY ARRAY['intended_recipient','actual_receiver'] LOOP
            IF value#>>ARRAY[field,'kind']='known' THEN persons:=persons||jsonb_build_array(value#>ARRAY[field,'value']); END IF;
        END LOOP;
        IF value#>>'{representation,kind}'='declared' THEN
            persons:=persons||jsonb_build_array(value#>'{representation,represented}',value#>'{representation,representative}');
        END IF;
    END IF;
    SELECT COALESCE(jsonb_agg(item ORDER BY (item->>'id')::uuid,(item->>'revision')::bigint),'[]') INTO expected
        FROM (SELECT DISTINCT jsonb_build_object('id',p->'id','revision',p->'revision') item
            FROM jsonb_array_elements(persons) p WHERE p->>'kind'='participant') selected_people;
    SELECT COALESCE(jsonb_agg(jsonb_build_object('id',p->'id','revision',p->'revision') ORDER BY ordinal),'[]') INTO actual
        FROM jsonb_array_elements(sources->'participants') WITH ORDINALITY a(p,ordinal);
    IF expected IS DISTINCT FROM actual THEN
        RAISE EXCEPTION 'participant sources differ from exact immediate selection' USING ERRCODE='23514'; END IF;
    FOR selected IN SELECT jsonb_array_elements(sources->'participants') LOOP
        SELECT count(*) INTO source_count FROM (
            SELECT r.revision FROM %1$I.case_participant_revisions r JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=scope AND p.id=(selected->>'id')::uuid AND r.revision=(selected->>'revision')::bigint
            UNION ALL SELECT r.revision FROM %1$I.case_participant_typed_revisions r JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=scope AND p.id=(selected->>'id')::uuid AND r.revision=(selected->>'revision')::bigint) revisions;
        IF source_count<>1 THEN RAISE EXCEPTION 'participant must have one exact historical revision' USING ERRCODE='23514'; END IF;
        SELECT * INTO manual_row FROM %1$I.case_participant_revisions
            WHERE participant_id=(selected->>'id')::uuid AND revision=(selected->>'revision')::bigint;
        IF FOUND THEN
            IF manual_row.values_digest IS DISTINCT FROM sha256(%1$I.participant_values_bytes(manual_row.display_name,
                manual_row.procedural_role,manual_row.organization,manual_row.legal_status,manual_row.directory_status)) THEN
                RAISE EXCEPTION 'manual participant values are inconsistent' USING ERRCODE='23514'; END IF;
            expected:=jsonb_build_object('case',scope,'id',manual_row.participant_id,'revision',manual_row.revision,
                'values_digest',encode(manual_row.values_digest,'hex'),'status',manual_row.directory_status,
                'subject',NULL,'kind',NULL,'display_name',manual_row.display_name,
                'procedural_role',manual_row.procedural_role,'organization',manual_row.organization);
        ELSE
            SELECT * INTO typed_row FROM %1$I.case_participant_typed_revisions
                WHERE participant_id=(selected->>'id')::uuid AND revision=(selected->>'revision')::bigint;
            values_json:=%1$I.typed_participant_values(typed_row.values_canonical);
            SELECT r.*,s.case_id INTO subject_row FROM %1$I.case_subject_revisions r
                JOIN %1$I.case_subjects s ON s.id=r.subject_id
                WHERE r.subject_id=typed_row.subject_id AND r.revision=typed_row.subject_revision;
            IF NOT FOUND OR subject_row.case_id IS DISTINCT FROM scope
                OR typed_row.values_digest IS DISTINCT FROM sha256(typed_row.values_canonical)
                OR values_json IS DISTINCT FROM typed_row.values_view
                OR subject_row.values_digest IS DISTINCT FROM sha256(subject_row.values_canonical)
                OR subject_row.values_digest IS DISTINCT FROM decode(values_json#>>'{subject,digest}','hex')
                OR typed_row.subject_id IS DISTINCT FROM (values_json#>>'{subject,id}')::uuid
                OR typed_row.subject_revision IS DISTINCT FROM (values_json#>>'{subject,revision}')::bigint
                OR typed_row.role_kind IS DISTINCT FROM values_json#>>'{profile,kind}'
                OR typed_row.organization IS DISTINCT FROM values_json->>'organization'
                OR typed_row.directory_status IS DISTINCT FROM values_json->>'directory_status' THEN
                RAISE EXCEPTION 'typed participant exact identity is inconsistent' USING ERRCODE='23514'; END IF;
            subject_json:=%1$I.typed_subject_values(subject_row.values_canonical);
            IF subject_json IS DISTINCT FROM subject_row.values_view
                OR (typed_row.role_kind='trial_court' AND subject_json->>'kind'<>'institutional_body')
                OR (typed_row.role_kind NOT IN ('trial_court','other') AND subject_json->>'kind'<>'natural_person') THEN
                RAISE EXCEPTION 'typed participant subject projection or kind differs' USING ERRCODE='23514'; END IF;
            expected:=jsonb_build_object('case',scope,'id',typed_row.participant_id,'revision',typed_row.revision,
                'values_digest',encode(typed_row.values_digest,'hex'),'status',typed_row.directory_status,
                'subject',jsonb_build_object('id',typed_row.subject_id,'revision',typed_row.subject_revision,
                    'values_digest',encode(subject_row.values_digest,'hex')),'kind',typed_row.role_kind,'procedural_role',typed_row.role_kind,
                'organization',typed_row.organization,'display_name',CASE subject_json->>'kind' WHEN 'natural_person'
                    THEN COALESCE(subject_json#>>'{name,known}',subject_json#>>'{name,label}') ELSE subject_json->>'name' END);
        END IF;
        IF expected IS DISTINCT FROM selected THEN
            RAISE EXCEPTION 'participant snapshot differs from exact historical values' USING ERRCODE='23514'; END IF;
    END LOOP;
    provenances:=jsonb_build_array(value->'provenance');
    IF family='notification' AND value#>>'{representation,kind}'='declared' THEN
        provenances:=provenances||jsonb_build_array(value#>'{representation,provenance}'); END IF;
    SELECT COALESCE(jsonb_agg(item ORDER BY (item->>'hearing_id')::uuid,(item->>'result_id')::uuid,
        (item->>'revision')::bigint,(item->>'agreement_id')::uuid NULLS FIRST),'[]') INTO expected
        FROM (SELECT DISTINCT p->'reference' item FROM jsonb_array_elements(provenances) p
            WHERE p->>'kind'='hearing_result') selected_results;
    SELECT COALESCE(jsonb_agg(jsonb_build_object('hearing_id',p->'hearing_id','result_id',p->'result_id',
        'revision',p->'revision','agreement_id',p->'agreement_id') ORDER BY ordinal),'[]') INTO actual
        FROM jsonb_array_elements(sources->'hearing_results') WITH ORDINALITY a(p,ordinal);
    IF expected IS DISTINCT FROM actual THEN
        RAISE EXCEPTION 'result sources differ from exact immediate selection' USING ERRCODE='23514'; END IF;
    FOR selected IN SELECT jsonb_array_elements(sources->'hearing_results') LOOP
        SELECT * INTO hearing_row FROM %1$I.case_hearing_result_revisions WHERE case_id=scope
            AND hearing_id=(selected->>'hearing_id')::uuid AND result_id=(selected->>'result_id')::uuid
            AND revision=(selected->>'revision')::bigint;
        IF NOT FOUND OR hearing_row.values_digest IS DISTINCT FROM sha256(hearing_row.values_canonical)
            OR hearing_row.submission_digest IS DISTINCT FROM sha256(hearing_row.submission_canonical) THEN
            RAISE EXCEPTION 'result source is absent or inconsistent' USING ERRCODE='23514'; END IF;
        values_json:=%1$I.hearing_result_values(hearing_row.values_canonical);
        IF values_json IS DISTINCT FROM hearing_row.values_view THEN
            RAISE EXCEPTION 'result source projection differs from canonical content' USING ERRCODE='23514'; END IF;
        agreement:=NULL;
        IF selected->>'agreement_id' IS NOT NULL THEN
            SELECT a->'text' INTO agreement FROM jsonb_array_elements(values_json->'agreements') a
                WHERE (a->>'id')::uuid=(selected->>'agreement_id')::uuid;
            IF NOT FOUND THEN RAISE EXCEPTION 'selected result agreement does not exist' USING ERRCODE='23514'; END IF;
        END IF;
        temporal:=values_json->'event_time';
        IF temporal->>'precision'='instant' THEN
            local_time:=TIMESTAMP '1970-01-01 00:00:00'
                +((temporal->>'seconds')::bigint+(temporal->>'offset_seconds')::integer)*INTERVAL '1 second';
            temporal:=(temporal-'seconds')||jsonb_build_object('date',to_char(local_time,'YYYY-MM-DD'),
                'hour',extract(hour FROM local_time)::integer,'minute',extract(minute FROM local_time)::integer,
                'second',extract(second FROM local_time)::integer);
        END IF;
        expected:=jsonb_build_object('case',scope,'hearing_id',hearing_row.hearing_id,'result_id',hearing_row.result_id,
            'revision',hearing_row.revision,'agreement_id',selected->'agreement_id',
            'values_digest',encode(hearing_row.values_digest,'hex'),'submission_digest',encode(hearing_row.submission_digest,'hex'),
            'status',hearing_row.status,'occurrence',values_json->'occurrence','event_time',temporal,
            'summary',values_json->'summary','agreement_text',agreement);
        IF expected IS DISTINCT FROM selected THEN
            RAISE EXCEPTION 'result snapshot differs from exact historical values' USING ERRCODE='23514'; END IF;
    END LOOP;
    SELECT COALESCE(jsonb_agg(item ORDER BY (item->>'document_id')::uuid,(item->>'version')::bigint),'[]') INTO support_refs
        FROM (SELECT DISTINCT jsonb_build_object('document_id',p#>'{support,document_id}',
            'version',p#>'{support,version}','digest',p#>'{support,digest}') item
            FROM jsonb_array_elements(provenances) p WHERE jsonb_typeof(p->'support')='object') selected_documents;
    SELECT COALESCE(jsonb_agg(jsonb_build_object('document_id',p->'id','version',p->'version',
        'digest',p->'digest') ORDER BY ordinal),'[]') INTO actual
        FROM jsonb_array_elements(sources->'direct_supports') WITH ORDINALITY a(p,ordinal);
    IF support_refs IS DISTINCT FROM actual THEN
        RAISE EXCEPTION 'document sources differ from exact immediate selection' USING ERRCODE='23514'; END IF;
    FOR selected IN SELECT jsonb_array_elements(sources->'direct_supports') LOOP
        IF NOT EXISTS(SELECT 1 FROM %1$I.documents WHERE case_id=scope AND id=(selected->>'id')::uuid
            AND version=(selected->>'version')::bigint AND digest=decode(selected->>'digest','hex')
            AND name=selected->>'name' COLLATE "C") THEN
            RAISE EXCEPTION 'document source differs from exact case content' USING ERRCODE='23514'; END IF;
    END LOOP;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION validate_procedural_fact_sources(UUID,TEXT,JSONB,JSONB) FROM PUBLIC;
