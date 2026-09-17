-- Civil dates and declared public references use an intentionally bounded profile.
CREATE OR REPLACE FUNCTION judicial_calendar_url_valid(value TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE rest TEXT; host TEXT; tail TEXT; labels TEXT[]; label TEXT;
BEGIN
    IF value IS NULL OR octet_length(value) NOT BETWEEN 1 AND 2048
        OR octet_length(value)<>char_length(value) OR left(value,8)<>'https://' THEN
        RETURN FALSE;
    END IF;
    rest:=substring(value FROM 9);
    host:=substring(rest FROM '^[^/?#]*');
    labels:=string_to_array(host,'.');
    IF octet_length(host)>253 OR coalesce(array_length(labels,1),0)<2 THEN RETURN FALSE; END IF;
    FOREACH label IN ARRAY labels LOOP
        IF label COLLATE "C" !~ '^[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?$'
            OR lower(left(label,4))='xn--' THEN RETURN FALSE; END IF;
    END LOOP;
    IF labels[array_length(labels,1)] COLLATE "C" !~ '^[A-Za-z]{2,63}$' THEN RETURN FALSE; END IF;
    tail:=substring(rest FROM char_length(host)+1);
    IF tail COLLATE "C" !~ '^[A-Za-z0-9._~!$&''()*+,;=:@/?#%-]*$'
        OR char_length(tail)-char_length(replace(tail,'#',''))>1
        OR position('%' IN regexp_replace(tail COLLATE "C",'%[0-9A-Fa-f]{2}','','g'))>0 THEN
        RETURN FALSE;
    END IF;
    RETURN TRUE;
END; $$;
CREATE OR REPLACE FUNCTION judicial_calendar_date(b BYTEA,p INTEGER)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE number INTEGER; day DATE; rendered TEXT;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 OR p+4>octet_length(b) THEN
        RAISE EXCEPTION 'truncated calendar civil date' USING ERRCODE='23514';
    END IF;
    number:=('x'||encode(substring(b FROM p+1 FOR 4),'hex'))::bit(32)::integer;
    IF number NOT BETWEEN -719162 AND 2932896 THEN
        RAISE EXCEPTION 'invalid calendar civil date' USING ERRCODE='23514';
    END IF;
    day:=DATE '1970-01-01'+number;
    rendered:=lpad(extract(year FROM day)::integer::text,4,'0')||'-'||
        lpad(extract(month FROM day)::integer::text,2,'0')||'-'||
        lpad(extract(day FROM day)::integer::text,2,'0');
    RETURN jsonb_build_object('next',p+4,'value',rendered);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR datetime_field_overflow THEN
    RAISE EXCEPTION 'invalid calendar civil date bytes' USING ERRCODE='23514';
END; $$;
REVOKE ALL ON FUNCTION judicial_calendar_url_valid(TEXT),judicial_calendar_date(BYTEA,INTEGER) FROM PUBLIC;
