-- PFSRC1 rejects reordering, duplicate keys and contradictory views of one result.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_sources(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=6; tag INTEGER; item JSONB; value JSONB; items JSONB; field TEXT;
    amount BIGINT; maximum INTEGER; kind TEXT; i INTEGER; prior_key BYTEA; source_key BYTEA;
    prior_value JSONB; current_value JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 19 AND 36847
        OR substring(b FROM 1 FOR 6)<>convert_to('PFSRC1','UTF8') THEN
        RAISE EXCEPTION 'invalid procedural fact sources header' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        item:=%1$I.procedural_fact_source_item(b,p,'resolution');p:=(item->>'next')::integer;
        value:=jsonb_build_object('resolution',item->'value');
    ELSIF tag=0 THEN value:=jsonb_build_object('resolution',NULL);
    ELSE RAISE EXCEPTION 'invalid procedural fact resolution option' USING ERRCODE='23514'; END IF;
    FOREACH field IN ARRAY ARRAY['participants','hearing_results','direct_supports'] LOOP
        maximum:=CASE field WHEN 'participants' THEN 4 ELSE 2 END;
        kind:=CASE field WHEN 'participants' THEN 'participant' WHEN 'hearing_results' THEN 'hearing' ELSE 'support' END;
        amount:=%1$I.typed_u32(b,p);p:=p+4;
        IF amount>maximum THEN RAISE EXCEPTION 'procedural fact source count exceeds limit' USING ERRCODE='23514'; END IF;
        items:='[]';prior_key:=NULL;prior_value:=NULL;
        FOR i IN 1..amount::integer LOOP
            item:=%1$I.procedural_fact_source_item(b,p,kind);p:=(item->>'next')::integer;
            source_key:=decode(item->>'key','hex');current_value:=item->'value';
            IF prior_key IS NOT NULL AND source_key<=prior_key THEN
                RAISE EXCEPTION 'procedural fact source keys are not strictly increasing' USING ERRCODE='23514'; END IF;
            IF kind='hearing' AND prior_value->>'hearing_id'=current_value->>'hearing_id'
                AND prior_value->>'result_id'=current_value->>'result_id'
                AND prior_value->>'revision'=current_value->>'revision'
                AND (prior_value-ARRAY['agreement_id','agreement_text'])
                    IS DISTINCT FROM (current_value-ARRAY['agreement_id','agreement_text']) THEN
                RAISE EXCEPTION 'contradictory procedural fact result views' USING ERRCODE='23514'; END IF;
            items:=items||jsonb_build_array(current_value);prior_key:=source_key;prior_value:=current_value;
        END LOOP;
        value:=value||jsonb_build_object(field,items);
    END LOOP;
    IF p<>octet_length(b) THEN
        RAISE EXCEPTION 'trailing procedural fact sources bytes' USING ERRCODE='23514'; END IF;
    RETURN value;
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character OR datetime_field_overflow THEN
    RAISE EXCEPTION 'invalid procedural fact sources bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_sources(BYTEA) FROM PUBLIC;
