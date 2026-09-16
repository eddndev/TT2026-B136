-- Strict finite primitives shared by the procedural fact canonical readers.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_atom(b BYTEA,p INTEGER,kind TEXT)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE tag INTEGER; width INTEGER; item JSONB; value JSONB; revision BIGINT;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 OR p>octet_length(b) OR kind IS NULL THEN
        RAISE EXCEPTION 'invalid procedural fact primitive' USING ERRCODE='23514';
    END IF;
    IF kind IN ('optional_uuid','optional_label','optional_text') THEN
        tag:=get_byte(b,p);p:=p+1;
        IF tag=0 THEN RETURN jsonb_build_object('next',p,'value',NULL); END IF;
        IF tag<>1 THEN RAISE EXCEPTION 'invalid procedural fact option' USING ERRCODE='23514'; END IF;
        RETURN %1$I.procedural_fact_atom(b,p,substring(kind FROM 10));
    END IF;
    IF kind IN ('label','text','name','role','filename') THEN
        width:=CASE kind WHEN 'text' THEN 1000 WHEN 'role' THEN 80
            WHEN 'filename' THEN 128 ELSE 200 END;
        item:=%1$I.hearing_text(b,p,width,kind='text');
        IF kind='filename' AND (octet_length(item->>'value')>128
            OR (item->>'value') !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$') THEN
            RAISE EXCEPTION 'unsafe procedural fact support name' USING ERRCODE='23514';
        END IF;
        RETURN item;
    END IF;
    width:=CASE kind WHEN 'uuid' THEN 16 WHEN 'digest' THEN 32 WHEN 'revision' THEN 4 END;
    IF width IS NULL OR p::bigint+width>octet_length(b) THEN
        RAISE EXCEPTION 'invalid procedural fact fixed primitive' USING ERRCODE='23514';
    END IF;
    IF kind='revision' THEN
        revision:=%1$I.typed_u32(b,p);
        IF revision=0 THEN RAISE EXCEPTION 'zero procedural fact revision' USING ERRCODE='23514'; END IF;
        value:=to_jsonb(revision);
    ELSIF kind='uuid' THEN value:=to_jsonb(encode(substring(b FROM p+1 FOR width),'hex')::uuid);
    ELSE value:=to_jsonb(encode(substring(b FROM p+1 FOR width),'hex')); END IF;
    RETURN jsonb_build_object('next',p+width,'value',value);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid procedural fact primitive bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_atom(BYTEA,INTEGER,TEXT) FROM PUBLIC;
