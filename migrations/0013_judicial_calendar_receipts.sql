-- JCTX1 binds the exact actor, operation, root, revision, values and stated reason.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.judicial_calendar_submission(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE action INTEGER; revision BIGINT; tag INTEGER; p INTEGER:=91; reason JSONB; item JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 91 AND 4095
        OR substring(b FROM 1 FOR 5)<>convert_to('JCTX1','UTF8') THEN
        RAISE EXCEPTION 'invalid calendar receipt header' USING ERRCODE='23514';
    END IF;
    action:=get_byte(b,53);revision:=%1$I.typed_u32(b,54);
    IF action NOT BETWEEN 0 AND 2 OR revision=4294967295
        OR (action=0 AND revision<>0) OR (action<>0 AND revision=0) THEN
        RAISE EXCEPTION 'invalid calendar receipt action or revision' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,90);
    IF tag=0 AND action=0 THEN reason:=NULL;
    ELSIF tag=1 AND action<>0 THEN
        item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;reason:=item->'value';
    ELSE RAISE EXCEPTION 'calendar receipt reason disagrees with action' USING ERRCODE='23514'; END IF;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing calendar receipt bytes' USING ERRCODE='23514'; END IF;
    RETURN jsonb_build_object('actor_id',encode(substring(b FROM 6 FOR 16),'hex')::uuid,
        'operation_id',encode(substring(b FROM 22 FOR 16),'hex')::uuid,
        'calendar_id',encode(substring(b FROM 38 FOR 16),'hex')::uuid,
        'action',(ARRAY['publish','replace','retire'])[action+1],'expected_revision',revision,
        'values_digest',encode(substring(b FROM 59 FOR 32),'hex'),'reason',reason);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid calendar receipt bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION judicial_calendar_submission(BYTEA) FROM PUBLIC;
