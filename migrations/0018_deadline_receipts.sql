-- Select the receipt version explicitly while preserving the exact DLTX1 projection.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_submission(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE action INTEGER; revision BIGINT; tag INTEGER; size BIGINT; reason TEXT;
BEGIN
    IF substring(b FROM 1 FOR 5)=convert_to('DLTX2','UTF8') THEN
        RETURN %1$I.deadline_submission_v2(b);
    END IF;
    IF b IS NULL OR octet_length(b) NOT BETWEEN 107 AND 4115
        OR substring(b FROM 1 FOR 5)<>convert_to('DLTX1','UTF8') THEN
        RAISE EXCEPTION 'invalid deadline receipt header' USING ERRCODE='23514';
    END IF;
    action:=get_byte(b,69);revision:=%1$I.typed_u32(b,70);tag:=get_byte(b,106);
    IF action NOT BETWEEN 0 AND 3 OR revision=4294967295
        OR (action=0 AND revision<>0) OR (action<>0 AND revision=0) THEN
        RAISE EXCEPTION 'invalid deadline receipt action or predecessor' USING ERRCODE='23514';
    END IF;
    IF tag=0 AND action=0 AND octet_length(b)=107 THEN reason:=NULL;
    ELSIF tag=1 AND action<>0 AND octet_length(b)>=115 THEN
        IF %1$I.typed_u32(b,107)<>0 THEN
            RAISE EXCEPTION 'deadline receipt text exceeds bound' USING ERRCODE='23514'; END IF;
        size:=%1$I.typed_u32(b,111);
        IF size>4000 OR size+115<>octet_length(b) THEN
            RAISE EXCEPTION 'invalid deadline receipt text length' USING ERRCODE='23514'; END IF;
        reason:=convert_from(substring(b FROM 116),'UTF8');
        IF NOT %1$I.case_administration_text_valid(reason,1000,TRUE) THEN
            RAISE EXCEPTION 'invalid deadline receipt reason' USING ERRCODE='23514'; END IF;
    ELSE
        RAISE EXCEPTION 'deadline receipt reason disagrees with action' USING ERRCODE='23514';
    END IF;
    RETURN jsonb_build_object('actor_id',encode(substring(b FROM 6 FOR 16),'hex')::uuid,
        'case_id',encode(substring(b FROM 22 FOR 16),'hex')::uuid,
        'deadline_id',encode(substring(b FROM 38 FOR 16),'hex')::uuid,
        'operation_id',encode(substring(b FROM 54 FOR 16),'hex')::uuid,
        'action',(ARRAY['register','correct','set_attention','retire'])[action+1],
        'expected_revision',revision,'review_digest',encode(substring(b FROM 75 FOR 32),'hex'),'reason',reason);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid deadline receipt bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_submission(BYTEA) FROM PUBLIC;
