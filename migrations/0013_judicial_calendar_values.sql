-- JCAL1 preserves complete bounded revisions independently of the Rust decoder.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.judicial_calendar_values(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=5; item JSONB; scope JSONB; title JSONB; jurisdiction INTEGER;
    n INTEGER; code INTEGER; last_code INTEGER:=0; codes JSONB:='[]'; key TEXT;
    first_day DATE; last_day DATE; coverage JSONB; sources JSONB:='[]'; source_ids UUID[]:='{}';
    previous UUID; id UUID; weekly JSONB:='[]'; exceptions JSONB:='[]'; rules JSONB:='[]'; rule JSONB;
    starts DATE; ends DATE; previous_end DATE; ids UUID[]:='{}'; reference TEXT;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 99 AND 191910
        OR substring(b FROM 1 FOR 5)<>convert_to('JCAL1','UTF8') THEN
        RAISE EXCEPTION 'invalid calendar canonical header' USING ERRCODE='23514';
    END IF;
    item:=%1$I.hearing_text(b,p,200,FALSE);p:=(item->>'next')::integer;title:=item->'value';
    jurisdiction:=get_byte(b,p);n:=get_byte(b,p+1);p:=p+2;
    IF jurisdiction NOT IN (0,1) OR n NOT BETWEEN 1 AND 32 THEN
        RAISE EXCEPTION 'invalid calendar scope jurisdiction or entities' USING ERRCODE='23514';
    END IF;
    FOR i IN 1..n LOOP
        code:=get_byte(b,p);p:=p+1;
        IF code NOT BETWEEN 1 AND 32 OR code<=last_code THEN
            RAISE EXCEPTION 'calendar entity codes must be strictly ordered' USING ERRCODE='23514';
        END IF;
        last_code:=code;codes:=codes||jsonb_build_array(lpad(code::text,2,'0'));
    END LOOP;
    scope:=jsonb_build_object('title',title,'jurisdiction',CASE jurisdiction WHEN 0 THEN 'federal' ELSE 'local' END,
        'entity_codes',codes);
    FOREACH key IN ARRAY ARRAY['authority','organ','territory','use_description'] LOOP
        item:=%1$I.hearing_text(b,p,CASE key WHEN 'use_description' THEN 1000 ELSE 200 END,key='use_description');
        p:=(item->>'next')::integer;scope:=scope||jsonb_build_object(key,item->'value');
    END LOOP;
    item:=%1$I.judicial_calendar_date(b,p);p:=(item->>'next')::integer;
    first_day:=(item->>'value')::date;coverage:=jsonb_build_object('from',item->'value');
    item:=%1$I.judicial_calendar_date(b,p);p:=(item->>'next')::integer;
    last_day:=(item->>'value')::date;coverage:=coverage||jsonb_build_object('through',item->'value');
    IF last_day-first_day NOT BETWEEN 0 AND 1095 THEN
        RAISE EXCEPTION 'invalid calendar coverage' USING ERRCODE='23514';
    END IF;
    n:=get_byte(b,p);p:=p+1;
    IF n>16 THEN RAISE EXCEPTION 'too many calendar sources' USING ERRCODE='23514'; END IF;
    FOR i IN 1..n LOOP
        item:=%1$I.judicial_calendar_source(b,p);p:=(item->>'next')::integer;id:=(item->'value'->>'id')::uuid;
        IF previous IS NOT NULL AND id<=previous THEN
            RAISE EXCEPTION 'calendar sources must be strictly ordered' USING ERRCODE='23514';
        END IF;
        previous:=id;source_ids:=array_append(source_ids,id);sources:=sources||jsonb_build_array(item->'value');
    END LOOP;
    FOR i IN 1..7 LOOP
        IF get_byte(b,p)<>i THEN
            RAISE EXCEPTION 'calendar weekdays must be complete and ordered' USING ERRCODE='23514';
        END IF;
        p:=p+1;item:=%1$I.judicial_calendar_rule(b,p);p:=(item->>'next')::integer;rule:=item->'value';
        rules:=rules||jsonb_build_array(rule);weekly:=weekly||jsonb_build_array(rule||jsonb_build_object('weekday',i));
    END LOOP;
    n:=get_byte(b,p);p:=p+1;
    IF n>64 THEN RAISE EXCEPTION 'too many calendar exceptions' USING ERRCODE='23514'; END IF;
    FOR i IN 1..n LOOP
        IF p+16>octet_length(b) THEN RAISE EXCEPTION 'truncated calendar exception' USING ERRCODE='23514'; END IF;
        id:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
        item:=%1$I.judicial_calendar_date(b,p);p:=(item->>'next')::integer;starts:=(item->>'value')::date;
        rule:=jsonb_build_object('id',id,'from',item->'value');
        item:=%1$I.judicial_calendar_date(b,p);p:=(item->>'next')::integer;ends:=(item->>'value')::date;
        rule:=rule||jsonb_build_object('through',item->'value');
        IF starts<first_day OR ends>last_day OR ends<starts OR starts<=previous_end OR id=ANY(ids) THEN
            RAISE EXCEPTION 'calendar exceptions must be unique ordered disjoint covered intervals' USING ERRCODE='23514';
        END IF;
        previous_end:=ends;ids:=array_append(ids,id);
        item:=%1$I.judicial_calendar_rule(b,p);p:=(item->>'next')::integer;
        rules:=rules||jsonb_build_array(item->'value');exceptions:=exceptions||jsonb_build_array(rule||(item->'value'));
    END LOOP;
    FOR rule IN SELECT jsonb_array_elements(rules) LOOP
        FOR reference IN SELECT jsonb_array_elements_text(rule->'source_ids') LOOP
            IF NOT(reference::uuid=ANY(source_ids)) THEN
                RAISE EXCEPTION 'calendar rule references missing revision source' USING ERRCODE='23514';
            END IF;
        END LOOP;
    END LOOP;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing calendar canonical bytes' USING ERRCODE='23514'; END IF;
    RETURN jsonb_build_object('scope',scope,'coverage',coverage,'sources',sources,'weekly_pattern',weekly,'exceptions',exceptions);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character OR datetime_field_overflow THEN
    RAISE EXCEPTION 'invalid calendar canonical bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION judicial_calendar_values(BYTEA) FROM PUBLIC;
