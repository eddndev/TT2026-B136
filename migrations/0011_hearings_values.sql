-- Decode bounded HEAR1 bytes independently of the Rust value implementation.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.hearing_text(b BYTEA,p INTEGER,maximum INTEGER,multiline BOOLEAN)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE n BIGINT; value TEXT;
BEGIN
    n:=%1$I.typed_u32(b,p);p:=p+4;
    IF maximum NOT BETWEEN 1 AND 1000 OR n>maximum*4 OR p+n>octet_length(b) THEN
        RAISE EXCEPTION 'invalid hearing canonical text size' USING ERRCODE='23514';
    END IF;
    value:=convert_from(substring(b FROM p+1 FOR n::integer),'UTF8');
    IF NOT %1$I.case_administration_text_valid(value,maximum,multiline) THEN
        RAISE EXCEPTION 'hearing text is not canonical' USING ERRCODE='23514';
    END IF;
    RETURN jsonb_build_object('next',p+n,'value',value);
EXCEPTION WHEN character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid hearing canonical text' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.hearing_values(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE n INTEGER; p INTEGER:=19; tag INTEGER; kind INTEGER; mode INTEGER;
    seconds BIGINT; delta INTEGER; item JSONB; result JSONB; participants JSONB:='[]';
    id UUID; previous UUID; revision BIGINT; basis JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 27 AND 10726
        OR substring(b FROM 1 FOR 5)<>convert_to('HEAR1','UTF8') THEN
        RAISE EXCEPTION 'invalid hearing canonical header' USING ERRCODE='23514';
    END IF;
    kind:=get_byte(b,5); mode:=get_byte(b,18);
    seconds:=('x'||encode(substring(b FROM 7 FOR 8),'hex'))::bit(64)::bigint;
    delta:=('x'||encode(substring(b FROM 15 FOR 4),'hex'))::bit(32)::integer;
    IF kind NOT BETWEEN 0 AND 3 OR mode NOT IN (0,1) OR delta NOT BETWEEN -50400 AND 50400
        OR delta %% 60<>0 OR seconds NOT BETWEEN -62135596800 AND 253402300799
        OR seconds+delta NOT BETWEEN -62135596800 AND 253402300799 THEN
        RAISE EXCEPTION 'invalid hearing kind, modality or time' USING ERRCODE='23514';
    END IF;
    result:=jsonb_build_object('kind',(ARRAY['initial','intermediate','oral_trial','sentencing'])[kind+1],
        'time',jsonb_build_object('seconds',seconds,'offset_seconds',delta),
        'modality',CASE mode WHEN 0 THEN 'in_person' ELSE 'videoconference' END);
    item:=%1$I.hearing_text(b,p,500,FALSE);p:=(item->>'next')::integer;
    result:=result||jsonb_build_object('venue',item->'value');
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN result:=result||jsonb_build_object('note',NULL);
    ELSIF tag=1 THEN
        item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;
        result:=result||jsonb_build_object('note',item->'value');
    ELSE RAISE EXCEPTION 'invalid hearing note tag' USING ERRCODE='23514'; END IF;
    n:=get_byte(b,p);p:=p+1;
    IF n>32 OR p+n*20>=octet_length(b) THEN
        RAISE EXCEPTION 'invalid hearing participant count' USING ERRCODE='23514';
    END IF;
    FOR i IN 1..n LOOP
        id:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;
        revision:=%1$I.typed_u32(b,p+16);p:=p+20;
        IF revision=0 OR (previous IS NOT NULL AND id<=previous) THEN
            RAISE EXCEPTION 'hearing participants must be exact and strictly ordered' USING ERRCODE='23514';
        END IF;
        participants:=participants||jsonb_build_array(jsonb_build_object('id',id,'revision',revision));
        previous:=id;
    END LOOP;
    result:=result||jsonb_build_object('participants',participants);
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 AND kind<>3 THEN basis:=NULL;
    ELSIF tag=1 AND kind=3 THEN
        item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;
        IF p+52>octet_length(b) THEN
            RAISE EXCEPTION 'truncated hearing conviction support' USING ERRCODE='23514';
        END IF;
        revision:=%1$I.typed_u32(b,p+16);
        IF revision=0 THEN RAISE EXCEPTION 'invalid hearing support version' USING ERRCODE='23514'; END IF;
        basis:=jsonb_build_object('statement',item->'value',
            'document_id',encode(substring(b FROM p+1 FOR 16),'hex')::uuid,
            'version',revision,'digest',encode(substring(b FROM p+21 FOR 32),'hex'));
        p:=p+52;
    ELSE RAISE EXCEPTION 'hearing conviction basis does not match kind' USING ERRCODE='23514'; END IF;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing hearing canonical bytes' USING ERRCODE='23514'; END IF;
    RETURN result||jsonb_build_object('conviction_basis',basis);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid hearing canonical bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION hearing_text(BYTEA,INTEGER,INTEGER,BOOLEAN),hearing_values(BYTEA) FROM PUBLIC;
