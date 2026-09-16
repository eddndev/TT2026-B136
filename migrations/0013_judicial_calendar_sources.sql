-- Decode declared references and rules without resolving remote resources.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.judicial_calendar_source(b BYTEA,p INTEGER)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE id UUID; title JSONB; issuer JSONB; url TEXT; n BIGINT; tag INTEGER;
    published JSONB; consulted JSONB; item JSONB; locator JSONB;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 OR p+16>octet_length(b) THEN
        RAISE EXCEPTION 'truncated calendar source' USING ERRCODE='23514';
    END IF;
    id:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
    item:=%1$I.hearing_text(b,p,200,FALSE);p:=(item->>'next')::integer;title:=item->'value';
    item:=%1$I.hearing_text(b,p,200,FALSE);p:=(item->>'next')::integer;issuer:=item->'value';
    n:=%1$I.typed_u32(b,p);p:=p+4;
    IF n NOT BETWEEN 1 AND 2048 OR p+n>octet_length(b) THEN
        RAISE EXCEPTION 'invalid calendar source URL size' USING ERRCODE='23514';
    END IF;
    url:=convert_from(substring(b FROM p+1 FOR n::integer),'UTF8');p:=p+n;
    IF NOT %1$I.judicial_calendar_url_valid(url) THEN
        RAISE EXCEPTION 'invalid calendar source URL' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN published:=NULL;
    ELSIF tag=1 THEN
        item:=%1$I.judicial_calendar_date(b,p);p:=(item->>'next')::integer;published:=item->'value';
    ELSE RAISE EXCEPTION 'invalid calendar publication option' USING ERRCODE='23514'; END IF;
    item:=%1$I.judicial_calendar_date(b,p);p:=(item->>'next')::integer;consulted:=item->'value';
    IF published IS NOT NULL AND published>consulted THEN
        RAISE EXCEPTION 'calendar publication follows consultation' USING ERRCODE='23514';
    END IF;
    item:=%1$I.hearing_text(b,p,512,FALSE);p:=(item->>'next')::integer;locator:=item->'value';
    RETURN jsonb_build_object('next',p,'value',jsonb_build_object('id',id,'title',title,
        'issuer',issuer,'official_url',url,'published_on',published,'consulted_on',consulted,'locator',locator));
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid calendar source bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.judicial_calendar_rule(b BYTEA,p INTEGER)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE classification INTEGER; n INTEGER; id UUID; previous UUID; refs JSONB:='[]'; item JSONB;
BEGIN
    IF b IS NULL OR p IS NULL OR p<0 OR p+2>octet_length(b) THEN
        RAISE EXCEPTION 'truncated calendar rule' USING ERRCODE='23514';
    END IF;
    classification:=get_byte(b,p);n:=get_byte(b,p+1);p:=p+2;
    IF classification NOT BETWEEN 0 AND 2 OR n>16 OR (classification<>2 AND n=0)
        OR p+n*16>octet_length(b) THEN
        RAISE EXCEPTION 'invalid calendar rule classification or references' USING ERRCODE='23514';
    END IF;
    FOR i IN 1..n LOOP
        id:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
        IF previous IS NOT NULL AND id<=previous THEN
            RAISE EXCEPTION 'calendar rule references must be strictly ordered' USING ERRCODE='23514';
        END IF;
        previous:=id;refs:=refs||jsonb_build_array(id);
    END LOOP;
    item:=%1$I.hearing_text(b,p,256,TRUE);p:=(item->>'next')::integer;
    RETURN jsonb_build_object('next',p,'value',jsonb_build_object(
        'classification',(ARRAY['countable','excluded','unresolved'])[classification+1],
        'source_ids',refs,'explanation',item->'value'));
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid calendar rule bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION judicial_calendar_source(BYTEA,INTEGER),judicial_calendar_rule(BYTEA,INTEGER) FROM PUBLIC;
