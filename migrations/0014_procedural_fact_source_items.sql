-- Readable source views are bound to the same bytes as their exact references.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_source_item(b BYTEA,p INTEGER,kind TEXT)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE value JSONB:='{}'; item JSONB; field TEXT; tag INTEGER; start_pos INTEGER:=p;
    source_key BYTEA; subject JSONB; names TEXT[]; kind_value TEXT;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 OR kind IS NULL
        OR kind NOT IN ('resolution','participant','hearing','support') THEN
        RAISE EXCEPTION 'invalid procedural fact source item input' USING ERRCODE='23514'; END IF;
    IF kind<>'support' THEN
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        value:=jsonb_build_object('case',item->'value');
    END IF;
    IF kind='hearing' THEN names:=ARRAY['hearing_id','result_id','revision','agreement_id'];
    ELSIF kind='support' THEN names:=ARRAY['id','version'];
    ELSE names:=ARRAY['id','revision']; END IF;
    FOREACH field IN ARRAY names LOOP
        item:=%1$I.procedural_fact_atom(b,p,CASE WHEN field IN ('revision','version') THEN 'revision'
            WHEN field='agreement_id' THEN 'optional_uuid' ELSE 'uuid' END);
        p:=(item->>'next')::integer;value:=value||jsonb_build_object(field,item->'value');
    END LOOP;
    source_key:=substring(b FROM start_pos+1+CASE kind WHEN 'support' THEN 0 ELSE 16 END
        FOR p-start_pos-CASE kind WHEN 'support' THEN 0 ELSE 16 END);
    IF kind IN ('resolution','hearing') THEN names:=ARRAY['values_digest','submission_digest'];
    ELSIF kind='participant' THEN names:=ARRAY['values_digest'];
    ELSE names:=ARRAY['digest']; END IF;
    FOREACH field IN ARRAY names LOOP
        item:=%1$I.procedural_fact_atom(b,p,'digest');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object(field,item->'value');
    END LOOP;
    IF kind<>'support' THEN
        tag:=get_byte(b,p);p:=p+1;
        IF tag>1 THEN RAISE EXCEPTION 'invalid procedural fact source status' USING ERRCODE='23514'; END IF;
        names:=CASE kind WHEN 'participant' THEN ARRAY['active','archived'] ELSE ARRAY['recorded','withdrawn'] END;
        value:=value||jsonb_build_object('status',names[tag+1]);
    END IF;
    IF kind='resolution' THEN
        FOREACH field IN ARRAY ARRAY['class','issuer'] LOOP
            item:=%1$I.procedural_fact_declaration(b,p,field);p:=(item->>'next')::integer;
            IF item#>>'{value,kind}'='unknown' THEN subject:=jsonb_build_object('unknown',item#>'{value,reason}');
            ELSE subject:=jsonb_build_object('known',item#>'{value,value}'); END IF;
            value:=value||jsonb_build_object(field,subject);
        END LOOP;
        item:=%1$I.procedural_fact_time(b,p,'source');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('issued_at',item->'value');
        item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('summary',item->'value');
    ELSIF kind='participant' THEN
        tag:=get_byte(b,p);p:=p+1;
        IF tag=1 THEN
            subject:='{}';
            FOREACH field IN ARRAY ARRAY['id','revision','values_digest'] LOOP
                item:=%1$I.procedural_fact_atom(b,p,CASE field WHEN 'id' THEN 'uuid'
                    WHEN 'revision' THEN 'revision' ELSE 'digest' END);
                p:=(item->>'next')::integer;subject:=subject||jsonb_build_object(field,item->'value');
            END LOOP;
        ELSIF tag<>0 THEN RAISE EXCEPTION 'invalid procedural fact subject option' USING ERRCODE='23514'; END IF;
        value:=value||jsonb_build_object('subject',subject);
        FOREACH field IN ARRAY ARRAY['display_name','procedural_role','organization'] LOOP
            item:=%1$I.procedural_fact_atom(b,p,CASE field WHEN 'display_name' THEN 'name'
                WHEN 'procedural_role' THEN 'role' ELSE 'optional_label' END);
            p:=(item->>'next')::integer;value:=value||jsonb_build_object(field,item->'value');
        END LOOP;
        tag:=get_byte(b,p);p:=p+1;
        IF tag=1 THEN
            tag:=get_byte(b,p);p:=p+1;
            names:=ARRAY['defendant','victim','defense_counsel','prosecutor','victim_counsel',
                'control_judge','trial_court','expert','police','precautionary_supervisor','other'];
            IF tag>10 THEN RAISE EXCEPTION 'invalid procedural fact participant kind' USING ERRCODE='23514'; END IF;
            kind_value:=names[tag+1];
        ELSIF tag<>0 THEN RAISE EXCEPTION 'invalid procedural fact kind option' USING ERRCODE='23514'; END IF;
        IF (kind_value IS NULL)<>(subject IS NULL)
            OR (kind_value IS NOT NULL AND kind_value<>value->>'procedural_role') THEN
            RAISE EXCEPTION 'procedural fact kind differs from typed participant' USING ERRCODE='23514'; END IF;
        value:=value||jsonb_build_object('kind',kind_value);
    ELSIF kind='hearing' THEN
        tag:=get_byte(b,p);p:=p+1;
        IF tag>1 THEN RAISE EXCEPTION 'invalid procedural fact hearing occurrence' USING ERRCODE='23514'; END IF;
        value:=value||jsonb_build_object('occurrence',(ARRAY['occurred','not_started'])[tag+1]);
        item:=%1$I.procedural_fact_time(b,p,'hearing');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('event_time',item->'value');
        item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('summary',item->'value');
        item:=%1$I.procedural_fact_atom(b,p,'optional_text');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('agreement_text',item->'value');
        IF (value->>'agreement_id' IS NULL)<>(value->>'agreement_text' IS NULL) THEN
            RAISE EXCEPTION 'procedural fact agreement reference and text differ' USING ERRCODE='23514'; END IF;
    ELSE
        item:=%1$I.procedural_fact_atom(b,p,'filename');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('name',item->'value');
        tag:=get_byte(b,p);p:=p+1;
        IF tag>1 OR get_byte(b,p)<>0 THEN
            RAISE EXCEPTION 'invalid procedural fact support format or policy' USING ERRCODE='23514'; END IF;
        p:=p+1;
        value:=value||jsonb_build_object('format',(ARRAY['pdf','docx'])[tag+1],'policy','pdf_docx_v1');
    END IF;
    RETURN jsonb_build_object('next',p,'key',encode(source_key,'hex'),'value',value);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation THEN
    RAISE EXCEPTION 'invalid procedural fact source item bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_source_item(BYTEA,INTEGER,TEXT) FROM PUBLIC;
