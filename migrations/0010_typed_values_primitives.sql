-- Strict bounded readers for the SUBJ1 and PART2 canonical value formats.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_u32(b BYTEA, p INTEGER)
RETURNS BIGINT LANGUAGE plpgsql IMMUTABLE AS $$
BEGIN
    IF b IS NULL OR p<0 OR p+4>octet_length(b) THEN
        RAISE EXCEPTION 'invalid typed canonical integer' USING ERRCODE='23514';
    END IF;
    RETURN (get_byte(b,p)::bigint<<24)+(get_byte(b,p+1)::bigint<<16)
        +(get_byte(b,p+2)::bigint<<8)+get_byte(b,p+3);
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_read(b BYTEA, p INTEGER, kind TEXT)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE n BIGINT; v TEXT; result JSONB; nested JSONB; q INTEGER:=p; tag INTEGER;
BEGIN
    IF b IS NULL OR p<0 OR p>=octet_length(b) THEN
        RAISE EXCEPTION 'invalid typed canonical value' USING ERRCODE='23514';
    END IF;
    IF left(kind,9)='declared_' THEN
        tag:=get_byte(b,q);q:=q+1;
        IF tag=0 THEN
            nested:=%1$I.typed_read(b,q,substr(kind,10));
            RETURN jsonb_build_object('next',nested->'next','value',jsonb_build_object('known',nested->'value'));
        ELSIF tag=1 THEN
            nested:=%1$I.typed_read(b,q,'text500');
            RETURN jsonb_build_object('next',nested->'next','value',jsonb_build_object('unknown',nested->'value'));
        END IF;
    ELSIF left(kind,9)='optional_' THEN
        tag:=get_byte(b,q);q:=q+1;
        IF tag=0 THEN RETURN jsonb_build_object('next',q,'value',NULL);
        ELSIF tag=1 THEN RETURN %1$I.typed_read(b,q,substr(kind,10)); END IF;
    ELSIF kind IN ('contact','protection') THEN
        tag:=get_byte(b,q);q:=q+1;
        IF tag IN (0,1) THEN
            nested:=%1$I.typed_read(b,q,'text500');
            RETURN jsonb_build_object('next',nested->'next','value',jsonb_build_object(CASE tag WHEN 0 THEN 'unknown' ELSE 'none' END,nested->'value'));
        ELSIF tag=2 THEN
            nested:=%1$I.typed_read(b,q,'locator');
            RETURN jsonb_build_object('next',nested->'next','value',jsonb_build_object('documented',nested->'value'));
        END IF;
    ELSIF kind='license' THEN
        nested:=%1$I.typed_read(b,q,'text32');q:=(nested->>'next')::integer;v:=nested->>'value';
        IF v COLLATE "C" !~ '^[A-Za-z0-9]{1,32}$' THEN
            RAISE EXCEPTION 'invalid canonical professional identifier' USING ERRCODE='23514';
        END IF;
        result:=jsonb_build_object('number',v);
        nested:=%1$I.typed_read(b,q,'text200');
        RETURN jsonb_build_object('next',nested->'next','value',result||jsonb_build_object('issuer',nested->'value'));
    ELSIF kind='locator' THEN
        IF q+52>=octet_length(b) THEN
            RAISE EXCEPTION 'invalid canonical evidence reference' USING ERRCODE='23514';
        END IF;
        n:=%1$I.typed_u32(b,q+16);
        IF n=0 THEN RAISE EXCEPTION 'invalid document version' USING ERRCODE='23514'; END IF;
        result:=jsonb_build_object('document_id',encode(substring(b FROM q+1 FOR 16),'hex')::uuid,
            'version',n,'digest',encode(substring(b FROM q+21 FOR 32),'hex'));
        nested:=%1$I.typed_read(b,q+52,'text200');
        RETURN jsonb_build_object('next',nested->'next','value',result||jsonb_build_object('locator',nested->'value'));
    ELSIF kind IN ('custody','mode','composition') THEN
        tag:=get_byte(b,q);
        IF tag NOT IN (0,1) THEN RAISE EXCEPTION 'invalid canonical enum' USING ERRCODE='23514'; END IF;
        v:=CASE kind WHEN 'custody' THEN CASE tag WHEN 0 THEN 'at_liberty' ELSE 'detained' END
            WHEN 'mode' THEN CASE tag WHEN 0 THEN 'private' ELSE 'public' END
            ELSE CASE tag WHEN 0 THEN 'single' ELSE 'collegiate' END END;
        RETURN jsonb_build_object('next',q+1,'value',v);
    ELSIF left(kind,4)='text' OR kind='curp' THEN
        n:=%1$I.typed_u32(b,q);q:=q+4;
        IF n>2000 OR q+n>octet_length(b) THEN
            RAISE EXCEPTION 'invalid canonical text length' USING ERRCODE='23514';
        END IF;
        v:=convert_from(substring(b FROM q+1 FOR n::integer),'UTF8');
        IF NOT %1$I.participant_text_valid(v,CASE kind WHEN 'curp' THEN 18 ELSE substr(kind,5)::integer END) THEN
            RAISE EXCEPTION 'typed text is not canonical' USING ERRCODE='23514';
        END IF;
        IF kind='curp' AND v COLLATE "C" !~ '^[A-Z0-9]{18}$' THEN
            RAISE EXCEPTION 'declared identifier is not canonical' USING ERRCODE='23514';
        END IF;
        RETURN jsonb_build_object('next',q+n,'value',v);
    END IF;
    RAISE EXCEPTION 'invalid typed canonical discriminator' USING ERRCODE='23514';
EXCEPTION WHEN character_not_in_repertoire OR untranslatable_character OR array_subscript_error THEN
    RAISE EXCEPTION 'invalid canonical bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION typed_u32(BYTEA,INTEGER),typed_read(BYTEA,INTEGER,TEXT) FROM PUBLIC;
