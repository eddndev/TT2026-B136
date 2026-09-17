-- DPTX1 binds actor, operation, profile, action, predecessor, algorithm and definition.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_profile_submission(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE action INTEGER; revision BIGINT; algorithm INTEGER; tag INTEGER;
    p INTEGER:=92; reason JSONB; item JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 92 AND 4096
        OR substring(b FROM 1 FOR 5)<>convert_to('DPTX1','UTF8') THEN
        RAISE EXCEPTION 'invalid deadline profile receipt header' USING ERRCODE='23514';
    END IF;
    action:=get_byte(b,53);revision:=%1$I.typed_u32(b,54);algorithm:=get_byte(b,58);
    IF action NOT BETWEEN 0 AND 2 OR algorithm<>1 OR revision=4294967295
        OR (action=0 AND revision<>0) OR (action<>0 AND revision=0) THEN
        RAISE EXCEPTION 'invalid deadline profile receipt state or algorithm' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,91);
    IF tag=0 AND action=0 THEN reason:=NULL;
    ELSIF tag=1 AND action<>0 THEN
        item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;reason:=item->'value';
    ELSE
        RAISE EXCEPTION 'deadline profile receipt reason disagrees with action' USING ERRCODE='23514';
    END IF;
    IF p<>octet_length(b) THEN
        RAISE EXCEPTION 'trailing deadline profile receipt bytes' USING ERRCODE='23514';
    END IF;
    RETURN jsonb_build_object('actor_id',encode(substring(b FROM 6 FOR 16),'hex')::uuid,
        'operation_id',encode(substring(b FROM 22 FOR 16),'hex')::uuid,
        'profile_id',encode(substring(b FROM 38 FOR 16),'hex')::uuid,
        'action',(ARRAY['publish','replace','retire'])[action+1],'expected_revision',revision,
        'algorithm',algorithm,'definition_digest',encode(substring(b FROM 60 FOR 32),'hex'),'reason',reason);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid deadline profile receipt bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_profile_submission(BYTEA) FROM PUBLIC;
