DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_provenance(b BYTEA,p INTEGER)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE tag INTEGER; item JSONB; value JSONB; reference JSONB; support JSONB; field TEXT;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 THEN
        RAISE EXCEPTION 'invalid procedural fact provenance input' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN
        item:=%1$I.procedural_fact_atom(b,p,'text');
        RETURN jsonb_build_object('next',item->'next','value',
            jsonb_build_object('kind','operator_note','note',item->'value'));
    ELSIF tag=1 THEN
        item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
        value:=jsonb_build_object('kind','external_reference','reference',item->'value');
    ELSIF tag=2 THEN
        reference:='{}';
        FOREACH field IN ARRAY ARRAY['hearing_id','result_id','revision','agreement_id'] LOOP
            item:=%1$I.procedural_fact_atom(b,p,CASE field WHEN 'revision' THEN 'revision'
                WHEN 'agreement_id' THEN 'optional_uuid' ELSE 'uuid' END);
            p:=(item->>'next')::integer;reference:=reference||jsonb_build_object(field,item->'value');
        END LOOP;
        item:=%1$I.procedural_fact_atom(b,p,'label');p:=(item->>'next')::integer;
        value:=jsonb_build_object('kind','hearing_result','reference',reference,'locator',item->'value');
    ELSE RAISE EXCEPTION 'invalid procedural fact provenance tag' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        support:='{}';
        FOREACH field IN ARRAY ARRAY['document_id','version','digest','locator'] LOOP
            item:=%1$I.procedural_fact_atom(b,p,CASE field WHEN 'document_id' THEN 'uuid'
                WHEN 'version' THEN 'revision' WHEN 'digest' THEN 'digest' ELSE 'label' END);
            p:=(item->>'next')::integer;support:=support||jsonb_build_object(field,item->'value');
        END LOOP;
    ELSIF tag<>0 THEN RAISE EXCEPTION 'invalid procedural fact evidence option' USING ERRCODE='23514'; END IF;
    RETURN jsonb_build_object('next',p,'value',value||jsonb_build_object('support',support));
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range THEN
    RAISE EXCEPTION 'invalid procedural fact provenance bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_representation(b BYTEA,p INTEGER)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE tag INTEGER; item JSONB; value JSONB; field TEXT;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 THEN
        RAISE EXCEPTION 'invalid procedural fact representation input' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN
        item:=%1$I.procedural_fact_atom(b,p,'text');
        RETURN jsonb_build_object('next',item->'next','value',
            jsonb_build_object('kind','not_recorded','reason',item->'value'));
    ELSIF tag<>1 THEN
        RAISE EXCEPTION 'invalid procedural fact representation tag' USING ERRCODE='23514'; END IF;
    value:=jsonb_build_object('kind','declared');
    FOREACH field IN ARRAY ARRAY['represented','representative','scope','provenance'] LOOP
        IF field='scope' THEN item:=%1$I.procedural_fact_atom(b,p,'text');
        ELSIF field='provenance' THEN item:=%1$I.procedural_fact_provenance(b,p);
        ELSE item:=%1$I.procedural_fact_person(b,p); END IF;
        p:=(item->>'next')::integer;value:=value||jsonb_build_object(field,item->'value');
    END LOOP;
    RETURN jsonb_build_object('next',p,'value',value);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range THEN
    RAISE EXCEPTION 'invalid procedural fact representation bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_provenance(BYTEA,INTEGER) FROM PUBLIC;
REVOKE ALL ON FUNCTION procedural_fact_representation(BYTEA,INTEGER) FROM PUBLIC;
