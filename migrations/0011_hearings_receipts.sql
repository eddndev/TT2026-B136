-- A receipt identifies one exact actor, command, scope and resulting value digest.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.hearing_submission(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE action INTEGER; revision BIGINT; tag INTEGER; p INTEGER:=75; result JSONB;
    context JSONB; item JSONB; reason JSONB; case_revision BIGINT; stage_revision BIGINT;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 108 AND 4120
        OR substring(b FROM 1 FOR 5)<>convert_to('HTXN1','UTF8') THEN
        RAISE EXCEPTION 'invalid hearing receipt header' USING ERRCODE='23514';
    END IF;
    action:=get_byte(b,69);revision:=%1$I.typed_u32(b,70);tag:=get_byte(b,74);
    IF action NOT BETWEEN 0 AND 2 OR revision=4294967295
        OR (action=0 AND revision<>0) OR (action<>0 AND revision=0) THEN
        RAISE EXCEPTION 'invalid hearing receipt action or revision' USING ERRCODE='23514';
    END IF;
    IF tag=0 AND action=2 THEN context:=NULL;
    ELSIF tag=1 AND action<>2 THEN
        case_revision:=%1$I.typed_u32(b,p);stage_revision:=%1$I.typed_u32(b,p+4);p:=p+8;
        IF case_revision=0 OR stage_revision=0 THEN
            RAISE EXCEPTION 'invalid hearing receipt context revision' USING ERRCODE='23514';
        END IF;
        context:=jsonb_build_object('case_revision',case_revision,'stage_revision',stage_revision);
    ELSE RAISE EXCEPTION 'hearing receipt context disagrees with action' USING ERRCODE='23514'; END IF;
    IF p+32>=octet_length(b) THEN RAISE EXCEPTION 'truncated hearing receipt digest' USING ERRCODE='23514'; END IF;
    result:=jsonb_build_object('operation_id',encode(substring(b FROM 6 FOR 16),'hex')::uuid,
        'actor_id',encode(substring(b FROM 22 FOR 16),'hex')::uuid,
        'case_id',encode(substring(b FROM 38 FOR 16),'hex')::uuid,
        'hearing_id',encode(substring(b FROM 54 FOR 16),'hex')::uuid,
        'action',(ARRAY['schedule','replace','cancel'])[action+1],
        'expected_revision',revision,'expected_context',context,
        'values_digest',encode(substring(b FROM p+1 FOR 32),'hex'));
    p:=p+32;tag:=get_byte(b,p);p:=p+1;
    IF tag=0 AND action=0 THEN reason:=NULL;
    ELSIF tag=1 AND action<>0 THEN
        item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;reason:=item->'value';
    ELSE RAISE EXCEPTION 'hearing receipt reason disagrees with action' USING ERRCODE='23514'; END IF;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing hearing receipt bytes' USING ERRCODE='23514'; END IF;
    RETURN result||jsonb_build_object('reason',reason);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid hearing receipt bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION hearing_submission(BYTEA) FROM PUBLIC;
