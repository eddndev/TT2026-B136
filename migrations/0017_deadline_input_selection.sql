-- Read the full DEVI1 structure before projecting exact foreign-key selections.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_input_selection(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=5; tag INTEGER; family INTEGER; item JSONB; result JSONB;
    field TEXT; fields TEXT[]; atom_kind TEXT; group_index INTEGER; position INTEGER;
    count BIGINT; identifier UUID; seen UUID[]:='{}'::uuid[];
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 48 AND 98897
        OR substring(b FROM 1 FOR 5)<>convert_to('DEVI1','UTF8') THEN
        RAISE EXCEPTION 'invalid DEVI1 header or size' USING ERRCODE='23514';
    END IF;
    item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
    result:=jsonb_build_object('case_id',item->'value',
        'source_kind',NULL,'source_id',NULL,'source_revision',NULL,
        'source_hearing_id',NULL,'source_parent_resolution_id',NULL,
        'source_parent_resolution_revision',NULL,'source_agreement_id',NULL,
        'calendar_id',NULL,'calendar_revision',NULL);
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN
        item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
    ELSIF tag=1 THEN
        family:=get_byte(b,p);p:=p+1;
        IF family=0 THEN
            result:=jsonb_set(result,'{source_kind}',to_jsonb('resolution'::text));
            fields:=ARRAY['source_id','source_revision'];
        ELSIF family=1 THEN
            result:=jsonb_set(result,'{source_kind}',to_jsonb('notification'::text));
            fields:=ARRAY['source_id','source_revision',
                'source_parent_resolution_id','source_parent_resolution_revision'];
        ELSIF family=2 THEN
            result:=jsonb_set(result,'{source_kind}',to_jsonb('hearing_result'::text));
            fields:=ARRAY['source_hearing_id','source_id','source_revision'];
        ELSE
            RAISE EXCEPTION 'invalid DEVI1 source family' USING ERRCODE='23514';
        END IF;
        FOREACH field IN ARRAY fields LOOP
            atom_kind:=CASE WHEN right(field,8)='revision' THEN 'revision' ELSE 'uuid' END;
            item:=%1$I.procedural_fact_atom(b,p,atom_kind);p:=(item->>'next')::integer;
            result:=jsonb_set(result,ARRAY[field],item->'value');
        END LOOP;
        IF family=2 THEN
            item:=%1$I.procedural_fact_atom(b,p,'optional_uuid');p:=(item->>'next')::integer;
            result:=jsonb_set(result,'{source_agreement_id}',item->'value');
        END IF;
    ELSE
        RAISE EXCEPTION 'invalid DEVI1 source declaration' USING ERRCODE='23514';
    END IF;

    -- Optional qualification remains syntactic, even for an unknown source.
    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        tag:=get_byte(b,p);p:=p+1;
        IF tag NOT BETWEEN 0 AND 1 THEN
            RAISE EXCEPTION 'invalid DEVI1 qualification purpose' USING ERRCODE='23514';
        END IF;
        item:=%1$I.procedural_fact_time(b,p,'fact');p:=(item->>'next')::integer;
        item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
        item:=%1$I.procedural_fact_atom(b,p,'label');p:=(item->>'next')::integer;
    ELSIF tag<>0 THEN
        RAISE EXCEPTION 'invalid DEVI1 qualification option' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        result:=jsonb_set(result,'{calendar_id}',item->'value');
        item:=%1$I.procedural_fact_atom(b,p,'revision');p:=(item->>'next')::integer;
        result:=jsonb_set(result,'{calendar_revision}',item->'value');
    ELSIF tag<>0 THEN
        RAISE EXCEPTION 'invalid DEVI1 calendar option' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        item:=%1$I.procedural_fact_atom(b,p,'revision');p:=(item->>'next')::integer;
    ELSIF tag<>0 THEN
        RAISE EXCEPTION 'invalid DEVI1 ordered quantity option' USING ERRCODE='23514';
    END IF;
    item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
    item:=%1$I.procedural_fact_atom(b,p,'label');p:=(item->>'next')::integer;

    -- Scope, incident and each ordered condition use the same three-state grammar.
    FOR group_index IN 0..2 LOOP
        count:=1;
        IF group_index=2 THEN
            count:=%1$I.typed_u32(b,p);p:=p+4;
            IF count>16 THEN
                RAISE EXCEPTION 'DEVI1 condition count exceeds sixteen' USING ERRCODE='23514';
            END IF;
        END IF;
        FOR position IN 1..count LOOP
            IF group_index=2 THEN
                item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
                identifier:=(item->>'value')::uuid;
                IF identifier=ANY(seen) THEN
                    RAISE EXCEPTION 'duplicate DEVI1 condition identifier' USING ERRCODE='23514';
                END IF;
                seen:=array_append(seen,identifier);
            END IF;
            tag:=get_byte(b,p);p:=p+1;
            IF tag=0 THEN
                item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
            ELSIF tag=1 THEN
                tag:=get_byte(b,p);p:=p+1;
                IF tag NOT BETWEEN 0 AND 1 THEN
                    RAISE EXCEPTION 'invalid DEVI1 declared boolean' USING ERRCODE='23514';
                END IF;
            ELSE
                RAISE EXCEPTION 'invalid DEVI1 boolean declaration' USING ERRCODE='23514';
            END IF;
            IF group_index=2 THEN
                item:=%1$I.procedural_fact_atom(b,p,'label');p:=(item->>'next')::integer;
            END IF;
        END LOOP;
    END LOOP;
    IF p<>octet_length(b) THEN
        RAISE EXCEPTION 'trailing DEVI1 bytes' USING ERRCODE='23514';
    END IF;
    RETURN result;
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character OR datetime_field_overflow THEN
    RAISE EXCEPTION 'invalid DEVI1 bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_input_selection(BYTEA) FROM PUBLIC;
