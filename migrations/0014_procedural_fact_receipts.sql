-- PFTXN1 binds exact values and sources to one actor, operation and target.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_submission(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=6; item JSONB; value JSONB:='{}'; field TEXT; family INTEGER;
    action INTEGER; revision BIGINT; reason JSONB; marker INTEGER;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 141 AND 4161
        OR substring(b FROM 1 FOR 6)<>convert_to('PFTXN1','UTF8') THEN
        RAISE EXCEPTION 'invalid procedural fact receipt header' USING ERRCODE='23514'; END IF;
    FOREACH field IN ARRAY ARRAY['operation_id','actor_id','case_id'] LOOP
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object(field,item->'value');
    END LOOP;
    family:=get_byte(b,p);p:=p+1;
    IF family>1 OR (family=0 AND octet_length(b) NOT BETWEEN 141 AND 4145)
        OR (family=1 AND octet_length(b) NOT BETWEEN 157 AND 4161) THEN
        RAISE EXCEPTION 'invalid procedural fact receipt family' USING ERRCODE='23514'; END IF;
    value:=value||jsonb_build_object('family',(ARRAY['resolution','notification'])[family+1]);
    item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
    value:=value||jsonb_build_object('target_id',item->'value','parent_id',NULL);
    IF family=1 THEN
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('parent_id',item->'value');
    END IF;
    action:=get_byte(b,p);revision:=%1$I.typed_u32(b,p+1);p:=p+5;
    IF action>2 OR revision=4294967295 OR (action=0 AND revision<>0) OR (action<>0 AND revision=0) THEN
        RAISE EXCEPTION 'invalid procedural fact receipt action or revision' USING ERRCODE='23514'; END IF;
    value:=value||jsonb_build_object('action',(ARRAY['record','correct','withdraw'])[action+1],
        'expected_revision',revision);
    FOREACH field IN ARRAY ARRAY['values_digest','sources_digest'] LOOP
        item:=%1$I.procedural_fact_atom(b,p,'digest');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object(field,item->'value');
    END LOOP;
    marker:=get_byte(b,p);p:=p+1;
    IF marker=1 AND action<>0 THEN
        item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;reason:=item->'value';
    ELSIF marker<>0 OR action<>0 THEN
        RAISE EXCEPTION 'procedural fact receipt reason differs from action' USING ERRCODE='23514'; END IF;
    IF p<>octet_length(b) THEN
        RAISE EXCEPTION 'trailing procedural fact receipt bytes' USING ERRCODE='23514'; END IF;
    RETURN value||jsonb_build_object('reason',reason);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid procedural fact receipt bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_submission(BYTEA) FROM PUBLIC;
