-- Exact query keys leave every historical canonical timestamp and chain intact.
CREATE OR REPLACE FUNCTION audit_timestamp_parts(value TEXT) RETURNS BIGINT[]
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE SET search_path = pg_catalog AS $function$
DECLARE
    y INTEGER; m INTEGER; d INTEGER; h INTEGER; minute INTEGER; second INTEGER;
    day DATE; seconds BIGINT; nanos BIGINT := 0; fraction TEXT;
BEGIN
    IF octet_length(value) NOT BETWEEN 20 AND 30
        OR value !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}([.][0-9]{0,8}[1-9])?Z$' THEN
        RAISE EXCEPTION 'audit timestamp is not canonical UTC' USING ERRCODE = '23514';
    END IF;
    y := substring(value FROM 1 FOR 4)::INTEGER;
    m := substring(value FROM 6 FOR 2)::INTEGER;
    d := substring(value FROM 9 FOR 2)::INTEGER;
    h := substring(value FROM 12 FOR 2)::INTEGER;
    minute := substring(value FROM 15 FOR 2)::INTEGER;
    second := substring(value FROM 18 FOR 2)::INTEGER;
    IF h NOT BETWEEN 0 AND 23 OR minute NOT BETWEEN 0 AND 59 OR second NOT BETWEEN 0 AND 59 THEN
        RAISE EXCEPTION 'audit time is outside its canonical range' USING ERRCODE = '23514';
    END IF;
    -- RFC 3339 year zero is astronomical year zero, PostgreSQL's 1 BC.
    day := make_date(CASE WHEN y=0 THEN -1 ELSE y END,m,d);
    seconds := (day - DATE '1970-01-01')::BIGINT * 86400 + h * 3600 + minute * 60 + second;
    IF length(value)>20 THEN
        fraction := substring(value FROM 21 FOR length(value)-21);
        nanos := rpad(fraction,9,'0')::BIGINT;
    END IF;
    RETURN ARRAY[seconds,nanos];
EXCEPTION WHEN datetime_field_overflow OR invalid_text_representation OR numeric_value_out_of_range THEN
    RAISE EXCEPTION 'invalid canonical audit timestamp' USING ERRCODE = '23514';
END;
$function$;

ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS timestamp_seconds BIGINT
    GENERATED ALWAYS AS ((audit_timestamp_parts(timestamp))[1]) STORED NOT NULL;
ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS timestamp_nanos INTEGER
    GENERATED ALWAYS AS (((audit_timestamp_parts(timestamp))[2])::INTEGER) STORED NOT NULL;
CREATE INDEX IF NOT EXISTS audit_events_chronological
    ON audit_events(timestamp_seconds,timestamp_nanos,sequence);
REVOKE ALL ON FUNCTION audit_timestamp_parts(TEXT) FROM PUBLIC;
