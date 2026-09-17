-- Result times retain declared precision independently from scheduling.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.hearing_result_time(b BYTEA,p INTEGER)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE tag INTEGER; delta INTEGER; seconds BIGINT; first_second BIGINT; last_second BIGINT;
    day DATE; result JSONB;
BEGIN
    tag:=get_byte(b,p);
    IF tag=0 THEN
        IF p+9>octet_length(b) THEN RAISE EXCEPTION 'truncated result date' USING ERRCODE='23514'; END IF;
        day:=make_date(get_byte(b,p+1)*256+get_byte(b,p+2),get_byte(b,p+3),get_byte(b,p+4));
        delta:=('x'||encode(substring(b FROM p+6 FOR 4),'hex'))::bit(32)::integer;
        IF extract(year FROM day) NOT BETWEEN 1 AND 9999 THEN
            RAISE EXCEPTION 'invalid result date year' USING ERRCODE='23514';
        END IF;
        first_second:=(day-DATE '1970-01-01')::bigint*86400-delta;
        last_second:=first_second+86399;
        result:=jsonb_build_object('precision','date','date',
            lpad(extract(year FROM day)::integer::text,4,'0')||'-'||
            lpad(extract(month FROM day)::integer::text,2,'0')||'-'||
            lpad(extract(day FROM day)::integer::text,2,'0'),'offset_seconds',delta);
        p:=p+9;
    ELSIF tag=1 THEN
        IF p+13>octet_length(b) THEN RAISE EXCEPTION 'truncated result instant' USING ERRCODE='23514'; END IF;
        seconds:=('x'||encode(substring(b FROM p+2 FOR 8),'hex'))::bit(64)::bigint;
        delta:=('x'||encode(substring(b FROM p+10 FOR 4),'hex'))::bit(32)::integer;
        IF seconds+delta NOT BETWEEN -62135596800 AND 253402300799 THEN
            RAISE EXCEPTION 'invalid local result instant' USING ERRCODE='23514';
        END IF;
        first_second:=seconds;last_second:=seconds;
        result:=jsonb_build_object('precision','instant','seconds',seconds,'offset_seconds',delta);
        p:=p+13;
    ELSE RAISE EXCEPTION 'invalid result time precision' USING ERRCODE='23514'; END IF;
    IF delta NOT BETWEEN -50400 AND 50400 OR delta %% 60<>0
        OR first_second < -62135596800 OR last_second > 253402300799 THEN
        RAISE EXCEPTION 'result time is not representable in UTC' USING ERRCODE='23514';
    END IF;
    RETURN jsonb_build_object('next',p,'value',result);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR datetime_field_overflow THEN
    RAISE EXCEPTION 'invalid result time bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION hearing_result_time(BYTEA,INTEGER) FROM PUBLIC;
