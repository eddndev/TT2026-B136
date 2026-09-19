-- DLOB1 records exact observed references; evidence authenticity is checked separately.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_observations(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=22; size INTEGER; entry_count INTEGER; entry_index INTEGER;
    role_tag INTEGER; family_tag INTEGER; previous_role INTEGER:=-1; tag INTEGER;
    root_case UUID; identifier UUID; revision BIGINT; scoped_case UUID; hearing UUID;
    parent_id UUID; parent_revision BIGINT; source_parent_id UUID; source_parent_revision BIGINT;
    parent JSONB; entries JSONB:='[]'::jsonb;
BEGIN
    size:=octet_length(b);
    IF b IS NULL OR size NOT BETWEEN 111 AND 446
        OR substring(b FROM 1 FOR 5)<>convert_to('DLOB1','UTF8') THEN
        RAISE EXCEPTION 'invalid DLOB1 header or size' USING ERRCODE='23514';
    END IF;
    root_case:=encode(substring(b FROM 6 FOR 16),'hex')::uuid;
    entry_count:=get_byte(b,21);
    IF entry_count NOT BETWEEN 1 AND 4 THEN
        RAISE EXCEPTION 'invalid DLOB1 observation count' USING ERRCODE='23514';
    END IF;
    FOR entry_index IN 1..entry_count LOOP
        IF p+89>size THEN
            RAISE EXCEPTION 'truncated DLOB1 observation' USING ERRCODE='23514';
        END IF;
        role_tag:=get_byte(b,p);family_tag:=get_byte(b,p+1);p:=p+2;
        IF role_tag NOT BETWEEN 0 AND 3 OR family_tag NOT BETWEEN 0 AND 4
            OR role_tag<=previous_role OR (entry_index=1 AND role_tag<>0) THEN
            RAISE EXCEPTION 'invalid DLOB1 roles, families or order' USING ERRCODE='23514';
        END IF;
        previous_role:=role_tag;
        identifier:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
        revision:=%1$I.typed_u32(b,p);p:=p+4;
        IF revision=0 THEN
            RAISE EXCEPTION 'invalid DLOB1 observation revision' USING ERRCODE='23514';
        END IF;

        scoped_case:=NULL;hearing:=NULL;parent_id:=NULL;parent_revision:=NULL;parent:=NULL;
        tag:=get_byte(b,p);p:=p+1;
        IF tag=1 THEN
            IF p+16>size THEN
                RAISE EXCEPTION 'truncated DLOB1 case identifier' USING ERRCODE='23514';
            END IF;
            scoped_case:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
        ELSIF tag<>0 THEN
            RAISE EXCEPTION 'invalid DLOB1 case presence' USING ERRCODE='23514';
        END IF;
        tag:=get_byte(b,p);p:=p+1;
        IF tag=1 THEN
            IF p+16>size THEN
                RAISE EXCEPTION 'truncated DLOB1 hearing identifier' USING ERRCODE='23514';
            END IF;
            hearing:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
        ELSIF tag<>0 THEN
            RAISE EXCEPTION 'invalid DLOB1 hearing presence' USING ERRCODE='23514';
        END IF;
        tag:=get_byte(b,p);p:=p+1;
        IF tag=1 THEN
            IF p+20>size THEN
                RAISE EXCEPTION 'truncated DLOB1 parent reference' USING ERRCODE='23514';
            END IF;
            parent_id:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
            parent_revision:=%1$I.typed_u32(b,p);p:=p+4;
            IF parent_revision=0 THEN
                RAISE EXCEPTION 'invalid DLOB1 parent revision' USING ERRCODE='23514';
            END IF;
            parent:=jsonb_build_object('id',parent_id,'revision',parent_revision);
        ELSIF tag<>0 THEN
            RAISE EXCEPTION 'invalid DLOB1 parent presence' USING ERRCODE='23514';
        END IF;

        IF (role_tag=0 AND family_tag<>4) OR (role_tag=1 AND family_tag NOT BETWEEN 0 AND 2)
            OR (role_tag=2 AND family_tag<>3) OR (role_tag=3 AND family_tag<>0) THEN
            RAISE EXCEPTION 'DLOB1 role and family disagree' USING ERRCODE='23514';
        END IF;
        IF (family_tag=4 AND ((scoped_case IS NOT NULL AND scoped_case<>root_case) OR hearing IS NOT NULL))
            OR (family_tag=3 AND (scoped_case IS NOT NULL OR hearing IS NOT NULL))
            OR (family_tag IN (0,1) AND (scoped_case IS DISTINCT FROM root_case OR hearing IS NOT NULL))
            OR (family_tag=2 AND (scoped_case IS DISTINCT FROM root_case OR hearing IS NULL)) THEN
            RAISE EXCEPTION 'invalid DLOB1 dependency scope' USING ERRCODE='23514';
        END IF;
        IF (family_tag=1 AND parent_id IS NULL) OR (family_tag<>1 AND parent_id IS NOT NULL) THEN
            RAISE EXCEPTION 'invalid DLOB1 notification parent reference' USING ERRCODE='23514';
        END IF;
        IF role_tag=1 AND family_tag=1 THEN
            source_parent_id:=parent_id;source_parent_revision:=parent_revision;
        END IF;
        IF role_tag=3 AND (source_parent_id IS NULL OR identifier IS DISTINCT FROM source_parent_id
            OR revision<source_parent_revision) THEN
            RAISE EXCEPTION 'DLOB1 observed parent does not cover the exact notification parent' USING ERRCODE='23514';
        END IF;
        IF p+64>size THEN
            RAISE EXCEPTION 'truncated DLOB1 observation digests' USING ERRCODE='23514';
        END IF;
        entries:=entries||jsonb_build_array(jsonb_build_object(
            'role',(ARRAY['profile','source','calendar','notification_parent'])[role_tag+1],
            'family',(ARRAY['resolution','notification','hearing_result','calendar','profile'])[family_tag+1],
            'id',identifier,'revision',revision,'case_id',scoped_case,'hearing_id',hearing,
            'parent_resolution',parent,'submission_digest',encode(substring(b FROM p+1 FOR 32),'hex'),
            'evidence_digest',encode(substring(b FROM p+33 FOR 32),'hex')));
        p:=p+64;
    END LOOP;
    IF p<>size THEN
        RAISE EXCEPTION 'trailing DLOB1 bytes' USING ERRCODE='23514';
    END IF;
    -- A legacy capture may omit the observed parent head without losing its exact parent.
    RETURN jsonb_build_object('case_id',root_case,'entries',entries);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation THEN
    RAISE EXCEPTION 'invalid DLOB1 bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_observations(BYTEA) FROM PUBLIC;
