-- HRTX1 binds exact immutable sources and never encodes current administration.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.hearing_result_submission(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE p INTEGER:=159; action INTEGER; revision BIGINT; anchor_revision BIGINT;
    tag INTEGER; continuation JSONB; result JSONB; item JSONB; reason JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 192 AND 4296
        OR substring(b FROM 1 FOR 5)<>convert_to('HRTX1','UTF8') THEN
        RAISE EXCEPTION 'invalid result receipt header' USING ERRCODE='23514';
    END IF;
    action:=get_byte(b,85);revision:=%1$I.typed_u32(b,86);anchor_revision:=%1$I.typed_u32(b,90);
    IF action NOT BETWEEN 0 AND 2 OR revision=4294967295 OR anchor_revision=0
        OR (action=0 AND revision<>0) OR (action<>0 AND revision=0) THEN
        RAISE EXCEPTION 'invalid result receipt action or revision' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,158);
    IF tag=0 THEN continuation:=NULL;
    ELSIF tag=1 THEN
        IF p+100>octet_length(b) THEN RAISE EXCEPTION 'truncated result continuation' USING ERRCODE='23514'; END IF;
        IF %1$I.typed_u32(b,p+32)=0 THEN RAISE EXCEPTION 'invalid result continuation revision' USING ERRCODE='23514'; END IF;
        continuation:=jsonb_build_object('hearing_id',encode(substring(b FROM p+1 FOR 16),'hex')::uuid,
            'result_id',encode(substring(b FROM p+17 FOR 16),'hex')::uuid,
            'revision',%1$I.typed_u32(b,p+32),'values_digest',encode(substring(b FROM p+37 FOR 32),'hex'),
            'submission_digest',encode(substring(b FROM p+69 FOR 32),'hex'));p:=p+100;
    ELSE RAISE EXCEPTION 'invalid result continuation tag' USING ERRCODE='23514'; END IF;
    IF p+32>=octet_length(b) THEN RAISE EXCEPTION 'truncated result receipt values digest' USING ERRCODE='23514'; END IF;
    result:=jsonb_build_object('operation_id',encode(substring(b FROM 6 FOR 16),'hex')::uuid,
        'actor_id',encode(substring(b FROM 22 FOR 16),'hex')::uuid,
        'case_id',encode(substring(b FROM 38 FOR 16),'hex')::uuid,
        'hearing_id',encode(substring(b FROM 54 FOR 16),'hex')::uuid,
        'result_id',encode(substring(b FROM 70 FOR 16),'hex')::uuid,
        'action',(ARRAY['record','correct','withdraw'])[action+1],'expected_revision',revision,
        'anchor',jsonb_build_object('revision',anchor_revision,'values_digest',encode(substring(b FROM 95 FOR 32),'hex'),
            'submission_digest',encode(substring(b FROM 127 FOR 32),'hex')),
        'continuation',continuation,'values_digest',encode(substring(b FROM p+1 FOR 32),'hex'));
    p:=p+32;tag:=get_byte(b,p);p:=p+1;
    IF tag=0 AND action=0 THEN reason:=NULL;
    ELSIF tag=1 AND action<>0 THEN
        item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;reason:=item->'value';
    ELSE RAISE EXCEPTION 'result receipt reason disagrees with action' USING ERRCODE='23514'; END IF;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing result receipt bytes' USING ERRCODE='23514'; END IF;
    RETURN result||jsonb_build_object('reason',reason);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid result receipt bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION hearing_result_submission(BYTEA) FROM PUBLIC;
