-- A declaration preserves unknown precision and never adds a missing time component.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_attention_valid(value JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE at_value JSONB; precision TEXT; tag INTEGER; b BYTEA; field TEXT; n INTEGER;
    parsed JSONB;
BEGIN
    IF value IS NULL OR jsonb_typeof(value)<>'object' THEN RETURN FALSE; END IF;
    IF value=jsonb_build_object('status','pending') THEN RETURN TRUE; END IF;
    IF value->>'status' IS DISTINCT FROM 'recorded' OR jsonb_typeof(value->'statement') IS DISTINCT FROM 'string'
        OR jsonb_typeof(value->'locator') IS DISTINCT FROM 'string'
        OR NOT %1$I.case_administration_text_valid(value->>'statement',1000,TRUE)
        OR NOT %1$I.case_administration_text_valid(value->>'locator',200,FALSE)
        OR (value-'status'-'occurred_at'-'statement'-'locator')<>'{}'::jsonb THEN RETURN FALSE; END IF;
    at_value:=value->'occurred_at';
    IF at_value IS NULL OR jsonb_typeof(at_value)<>'object' THEN RETURN FALSE; END IF;
    IF at_value=jsonb_build_object('precision','unknown') THEN RETURN TRUE; END IF;
    precision:=at_value->>'precision';
    tag:=CASE precision WHEN 'date' THEN 1 WHEN 'minute' THEN 2 WHEN 'second' THEN 3 ELSE 0 END;
    IF tag=0 OR NOT(at_value ? 'offset_seconds') THEN RETURN FALSE; END IF;
    b:=set_byte(decode('00','hex'),0,tag);
    FOREACH field IN ARRAY ARRAY['year','month','day','hour','minute','second'] LOOP
        IF (field IN ('hour','minute') AND tag<2) OR (field='second' AND tag<3) THEN CONTINUE; END IF;
        IF jsonb_typeof(at_value->field) IS DISTINCT FROM 'number'
            OR (at_value->>field) !~ '^(0|[1-9][0-9]*)$' THEN RETURN FALSE; END IF;
        n:=(at_value->>field)::integer;
        IF field='year' THEN
            IF n NOT BETWEEN 1 AND 9999 THEN RETURN FALSE; END IF;
            b:=b||int2send(n::smallint);
        ELSE
            IF n NOT BETWEEN 0 AND 255 THEN RETURN FALSE; END IF;
            b:=b||set_byte(decode('00','hex'),0,n);
        END IF;
    END LOOP;
    IF at_value->'offset_seconds'='null'::jsonb THEN b:=b||decode('00','hex');
    ELSE
        IF jsonb_typeof(at_value->'offset_seconds') IS DISTINCT FROM 'number'
            OR (at_value->>'offset_seconds') !~ '^-?(0|[1-9][0-9]*)$' THEN RETURN FALSE; END IF;
        b:=b||decode('01','hex')||int4send((at_value->>'offset_seconds')::integer);
    END IF;
    parsed:=%1$I.procedural_fact_time(b,0,'fact');
    RETURN parsed->'value'=at_value AND (parsed->>'next')::integer=octet_length(b);
EXCEPTION WHEN OTHERS THEN
    RETURN FALSE;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_attention_valid(JSONB) FROM PUBLIC;
