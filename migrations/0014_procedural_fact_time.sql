-- Local precision is preserved; a missing offset never becomes UTC.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_time(b BYTEA,p INTEGER,kind TEXT)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE tag INTEGER; year INTEGER; month INTEGER; day INTEGER; hour INTEGER:=0;
    minute INTEGER:=0; second INTEGER:=0; marker INTEGER; offset_value BIGINT;
    date_value DATE; first_second BIGINT; last_second BIGINT; value JSONB; precision TEXT;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 OR kind IS NULL OR kind NOT IN ('fact','source','hearing') THEN
        RAISE EXCEPTION 'invalid procedural fact time input' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF kind='hearing' THEN
        IF tag NOT BETWEEN 0 AND 1 THEN
            RAISE EXCEPTION 'invalid hearing source precision' USING ERRCODE='23514'; END IF;
        tag:=CASE tag WHEN 0 THEN 1 ELSE 3 END;
    ELSIF tag=0 THEN
        RETURN jsonb_build_object('next',p,'value',jsonb_build_object('precision','unknown'));
    ELSIF tag NOT BETWEEN 1 AND 3 THEN
        RAISE EXCEPTION 'invalid procedural fact precision' USING ERRCODE='23514';
    END IF;
    year:=get_byte(b,p)*256+get_byte(b,p+1);month:=get_byte(b,p+2);day:=get_byte(b,p+3);p:=p+4;
    IF year NOT BETWEEN 1 AND 9999 THEN
        RAISE EXCEPTION 'procedural fact year outside range' USING ERRCODE='23514'; END IF;
    date_value:=make_date(year,month,day);
    IF tag>=2 THEN hour:=get_byte(b,p);minute:=get_byte(b,p+1);p:=p+2; END IF;
    IF tag=3 THEN second:=get_byte(b,p);p:=p+1; END IF;
    IF hour>23 OR minute>59 OR second>59 THEN
        RAISE EXCEPTION 'invalid procedural fact local time' USING ERRCODE='23514'; END IF;
    IF kind='hearing' THEN marker:=1;
    ELSE marker:=get_byte(b,p);p:=p+1; END IF;
    IF marker=1 THEN
        offset_value:=%1$I.typed_u32(b,p);p:=p+4;
        IF offset_value>=2147483648 THEN offset_value:=offset_value-4294967296; END IF;
        IF abs(offset_value)>50400 OR offset_value %% 60<>0 THEN
            RAISE EXCEPTION 'invalid procedural fact offset' USING ERRCODE='23514'; END IF;
        first_second:=(date_value-DATE '1970-01-01')::bigint*86400+hour*3600+minute*60+second-offset_value;
        last_second:=first_second+CASE tag WHEN 1 THEN 86399 WHEN 2 THEN 59 ELSE 0 END;
        IF first_second < -62135596800 OR last_second>253402300799 THEN
            RAISE EXCEPTION 'procedural fact UTC range outside supported years' USING ERRCODE='23514'; END IF;
    ELSIF marker<>0 THEN
        RAISE EXCEPTION 'invalid procedural fact offset option' USING ERRCODE='23514';
    END IF;
    precision:=CASE tag WHEN 1 THEN 'date' WHEN 2 THEN 'minute'
        ELSE CASE kind WHEN 'hearing' THEN 'instant' ELSE 'second' END END;
    value:=jsonb_build_object('precision',precision,'offset_seconds',offset_value);
    IF kind='fact' THEN value:=value||jsonb_build_object('year',year,'month',month,'day',day);
    ELSE value:=value||jsonb_build_object('date',to_char(date_value,'YYYY-MM-DD')); END IF;
    IF tag>=2 THEN value:=value||jsonb_build_object('hour',hour,'minute',minute); END IF;
    IF tag=3 THEN value:=value||jsonb_build_object('second',second); END IF;
    RETURN jsonb_build_object('next',p,'value',value);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR datetime_field_overflow THEN
    RAISE EXCEPTION 'invalid procedural fact time bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_time(BYTEA,INTEGER,TEXT) FROM PUBLIC;
