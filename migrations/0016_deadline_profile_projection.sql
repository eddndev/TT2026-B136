-- DPRF1 exposes only the title and immutable scope; Rust reproduces the full corpus.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_profile_definition(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=5; item JSONB; title JSONB; scope JSONB; tag INTEGER;
    jurisdiction INTEGER; n INTEGER; code INTEGER; last_code INTEGER:=0;
    codes JSONB:='[]'; key TEXT;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 202 AND 3261697
        OR substring(b FROM 1 FOR 5)<>convert_to('DPRF1','UTF8') THEN
        RAISE EXCEPTION 'invalid deadline profile canonical header' USING ERRCODE='23514';
    END IF;
    item:=%1$I.hearing_text(b,p,200,FALSE);p:=(item->>'next')::integer;title:=item->'value';
    item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        IF p+16>octet_length(b) THEN
            RAISE EXCEPTION 'truncated deadline profile case scope' USING ERRCODE='23514';
        END IF;
        scope:=jsonb_build_object('kind','case','case_id',encode(substring(b FROM p+1 FOR 16),'hex')::uuid);
    ELSIF tag=0 THEN
        item:=%1$I.hearing_text(b,p,200,FALSE);p:=(item->>'next')::integer;
        jurisdiction:=get_byte(b,p);n:=get_byte(b,p+1);p:=p+2;
        IF jurisdiction NOT IN (0,1) OR n NOT BETWEEN 1 AND 32 THEN
            RAISE EXCEPTION 'invalid deadline profile global scope' USING ERRCODE='23514';
        END IF;
        FOR i IN 1..n LOOP
            code:=get_byte(b,p);p:=p+1;
            IF code NOT BETWEEN 1 AND 32 OR code<=last_code THEN
                RAISE EXCEPTION 'deadline profile entities must be strictly ordered' USING ERRCODE='23514';
            END IF;
            last_code:=code;codes:=codes||jsonb_build_array(lpad(code::text,2,'0'));
        END LOOP;
        scope:=jsonb_build_object('title',item->'value',
            'jurisdiction',CASE jurisdiction WHEN 0 THEN 'federal' ELSE 'local' END,'entity_codes',codes);
        FOREACH key IN ARRAY ARRAY['authority','organ','territory','use_description'] LOOP
            item:=%1$I.hearing_text(b,p,CASE key WHEN 'use_description' THEN 1000 ELSE 200 END,key='use_description');
            p:=(item->>'next')::integer;scope:=scope||jsonb_build_object(key,item->'value');
        END LOOP;
        scope:=jsonb_build_object('kind','global','value',scope);
    ELSE
        RAISE EXCEPTION 'invalid deadline profile scope tag' USING ERRCODE='23514';
    END IF;
    RETURN jsonb_build_object('title',title,'scope',scope);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid deadline profile projection bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_profile_definition(BYTEA) FROM PUBLIC;
