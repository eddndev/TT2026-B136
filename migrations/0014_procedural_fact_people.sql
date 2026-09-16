DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_person(b BYTEA,p INTEGER)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE tag INTEGER; item JSONB; value JSONB; field TEXT;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 THEN
        RAISE EXCEPTION 'invalid procedural fact person input' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN
        value:=jsonb_build_object('kind','participant');
        FOREACH field IN ARRAY ARRAY['id','revision'] LOOP
            item:=%1$I.procedural_fact_atom(b,p,CASE field WHEN 'id' THEN 'uuid' ELSE 'revision' END);
            p:=(item->>'next')::integer;value:=value||jsonb_build_object(field,item->'value');
        END LOOP;
    ELSIF tag=1 THEN
        value:=jsonb_build_object('kind','unlinked');
        FOREACH field IN ARRAY ARRAY['label','description'] LOOP
            item:=%1$I.procedural_fact_atom(b,p,CASE field WHEN 'label' THEN 'label' ELSE 'text' END);
            p:=(item->>'next')::integer;value:=value||jsonb_build_object(field,item->'value');
        END LOOP;
    ELSE RAISE EXCEPTION 'invalid procedural fact person tag' USING ERRCODE='23514'; END IF;
    RETURN jsonb_build_object('next',p,'value',value);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range THEN
    RAISE EXCEPTION 'invalid procedural fact person bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_declaration(b BYTEA,p INTEGER,kind TEXT)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE tag INTEGER; item JSONB; value JSONB; names TEXT[];
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 OR kind IS NULL
        OR kind NOT IN ('class','character','medium','context','outcome','issuer','person') THEN
        RAISE EXCEPTION 'invalid procedural fact declaration input' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN
        item:=%1$I.procedural_fact_atom(b,p,'text');
        RETURN jsonb_build_object('next',item->'next','value',
            jsonb_build_object('kind','unknown','reason',item->'value'));
    ELSIF tag<>1 THEN
        RAISE EXCEPTION 'invalid procedural fact declaration tag' USING ERRCODE='23514'; END IF;
    IF kind='issuer' THEN item:=%1$I.procedural_fact_atom(b,p,'label');
    ELSIF kind='person' THEN item:=%1$I.procedural_fact_person(b,p);
    ELSE
        names:=CASE kind WHEN 'class' THEN ARRAY['order','judgment','other']
            WHEN 'character' THEN ARRAY['personal','publication','other']
            WHEN 'medium' THEN ARRAY['in_person','electronic','other']
            WHEN 'context' THEN ARRAY['in_hearing','outside_hearing','other']
            ELSE ARRAY['practiced','attempted'] END;
        tag:=get_byte(b,p);p:=p+1;
        IF tag>=array_length(names,1) THEN
            RAISE EXCEPTION 'invalid procedural fact declared class' USING ERRCODE='23514'; END IF;
        value:=jsonb_build_object('kind',names[tag+1]);
        IF tag=2 THEN
            item:=%1$I.procedural_fact_atom(b,p,'label');p:=(item->>'next')::integer;
            value:=value||jsonb_build_object('label',item->'value');
        END IF;
        item:=jsonb_build_object('next',p,'value',value);
    END IF;
    RETURN jsonb_build_object('next',item->'next','value',
        jsonb_build_object('kind','known','value',item->'value'));
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range THEN
    RAISE EXCEPTION 'invalid procedural fact declaration bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_person(BYTEA,INTEGER) FROM PUBLIC;
REVOKE ALL ON FUNCTION procedural_fact_declaration(BYTEA,INTEGER,TEXT) FROM PUBLIC;
