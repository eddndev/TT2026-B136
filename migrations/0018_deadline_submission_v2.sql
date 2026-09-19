-- DLTX2 commits explicit authors, observations and immutable predecessor receipts.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_submission_v2(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=122; action INTEGER; revision BIGINT; tag INTEGER; author_tag INTEGER;
    policy INTEGER; family INTEGER; high BIGINT; low BIGINT; sequence BIGINT;
    item JSONB; predecessor JSONB; author JSONB; cause JSONB; event JSONB;
    receipt_case UUID; actor_id UUID; job_id UUID; source_id UUID; event_revision BIGINT;
    event_case UUID; hearing_id UUID; event_operation UUID; email TEXT; reason TEXT;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 151 AND 5502
        OR substring(b FROM 1 FOR 5)<>convert_to('DLTX2','UTF8') THEN
        RAISE EXCEPTION 'invalid tracked deadline receipt header' USING ERRCODE='23514';
    END IF;
    receipt_case:=encode(substring(b FROM 6 FOR 16),'hex')::uuid;
    action:=get_byte(b,53);revision:=%1$I.typed_u32(b,54);
    IF action NOT BETWEEN 0 AND 4 OR revision=4294967295
        OR (action=0 AND revision<>0) OR (action<>0 AND revision=0) THEN
        RAISE EXCEPTION 'invalid tracked deadline action or revision' USING ERRCODE='23514';
    END IF;

    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        item:=%1$I.procedural_fact_atom(b,p,'digest');p:=(item->>'next')::integer;
        predecessor:=jsonb_build_object('submission_digest',item->'value');
        item:=%1$I.procedural_fact_atom(b,p,'digest');p:=(item->>'next')::integer;
        predecessor:=predecessor||jsonb_build_object('capture_digest',item->'value');
    ELSIF tag<>0 THEN
        RAISE EXCEPTION 'invalid tracked deadline predecessor tag' USING ERRCODE='23514';
    END IF;

    author_tag:=get_byte(b,p);p:=p+1;
    IF author_tag=0 THEN
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        actor_id:=(item->>'value')::uuid;
        -- The shared text reader consumes u32; DLTX2 first commits a zero high u32.
        IF %1$I.typed_u32(b,p)<>0 THEN
            RAISE EXCEPTION 'tracked deadline email length exceeds bound' USING ERRCODE='23514';
        END IF;
        item:=%1$I.hearing_text(b,p+4,320,FALSE);p:=(item->>'next')::integer;
        email:=item->>'value';
        author:=jsonb_build_object('kind','user','id',actor_id,'email',email);
    ELSIF author_tag=1 THEN
        tag:=get_byte(b,p);policy:=get_byte(b,p+1)*256+get_byte(b,p+2);p:=p+3;
        IF tag<>0 OR policy<>1 THEN
            RAISE EXCEPTION 'unsupported deadline technical author' USING ERRCODE='23514';
        END IF;
        author:=jsonb_build_object('kind','technical','service','deadline_reevaluator',
            'policy_version',policy);
    ELSE
        RAISE EXCEPTION 'invalid tracked deadline author tag' USING ERRCODE='23514';
    END IF;

    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        IF %1$I.typed_u32(b,p)<>0 THEN
            RAISE EXCEPTION 'tracked deadline reason length exceeds bound' USING ERRCODE='23514';
        END IF;
        item:=%1$I.hearing_text(b,p+4,1000,TRUE);p:=(item->>'next')::integer;
        reason:=item->>'value';
    ELSIF tag<>0 THEN
        RAISE EXCEPTION 'invalid tracked deadline reason tag' USING ERRCODE='23514';
    END IF;

    tag:=get_byte(b,p);p:=p+1;
    IF tag=1 THEN
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        job_id:=(item->>'value')::uuid;
        high:=%1$I.typed_u32(b,p);low:=%1$I.typed_u32(b,p+4);p:=p+8;
        IF high>2147483647 OR (high=0 AND low=0) THEN
            RAISE EXCEPTION 'deadline event sequence exceeds durable range' USING ERRCODE='23514';
        END IF;
        sequence:=(high<<32)+low;
        family:=get_byte(b,p);p:=p+1;
        IF family NOT BETWEEN 0 AND 4 THEN
            RAISE EXCEPTION 'invalid deadline event family' USING ERRCODE='23514';
        END IF;
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        source_id:=(item->>'value')::uuid;
        item:=%1$I.procedural_fact_atom(b,p,'revision');p:=(item->>'next')::integer;
        event_revision:=(item->>'value')::bigint;
        item:=%1$I.procedural_fact_atom(b,p,'optional_uuid');p:=(item->>'next')::integer;
        event_case:=(item->>'value')::uuid;
        item:=%1$I.procedural_fact_atom(b,p,'optional_uuid');p:=(item->>'next')::integer;
        hearing_id:=(item->>'value')::uuid;
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        event_operation:=(item->>'value')::uuid;
        IF (family IN (0,1) AND (event_case IS DISTINCT FROM receipt_case OR hearing_id IS NOT NULL))
            OR (family=2 AND (event_case IS DISTINCT FROM receipt_case OR hearing_id IS NULL))
            OR (family=3 AND (event_case IS NOT NULL OR hearing_id IS NOT NULL))
            OR (family=4 AND ((event_case IS NOT NULL AND event_case IS DISTINCT FROM receipt_case)
                OR hearing_id IS NOT NULL)) THEN
            RAISE EXCEPTION 'deadline event scope differs from its family or case' USING ERRCODE='23514';
        END IF;
        event:=jsonb_build_object('sequence',sequence,
            'family',(ARRAY['resolution','notification','hearing_result','calendar','profile'])[family+1],
            'source_id',source_id,'revision',event_revision,'case_id',event_case,
            'hearing_id',hearing_id,'operation_id',event_operation);
        cause:=jsonb_build_object('kind','source_event','job_id',job_id,'event',event);
    ELSIF tag=2 THEN
        item:=%1$I.procedural_fact_atom(b,p,'uuid');p:=(item->>'next')::integer;
        job_id:=(item->>'value')::uuid;
        policy:=get_byte(b,p)*256+get_byte(b,p+1);p:=p+2;
        IF policy<>1 THEN
            RAISE EXCEPTION 'unsupported deadline bootstrap policy' USING ERRCODE='23514';
        END IF;
        cause:=jsonb_build_object('kind','legacy_bootstrap','job_id',job_id,'policy_version',policy);
    ELSIF tag<>0 THEN
        RAISE EXCEPTION 'invalid tracked deadline cause tag' USING ERRCODE='23514';
    END IF;

    IF p<>octet_length(b) THEN
        RAISE EXCEPTION 'trailing tracked deadline receipt bytes' USING ERRCODE='23514';
    END IF;
    IF (action=0 AND (predecessor IS NOT NULL OR reason IS NOT NULL))
        OR (action<>0 AND (predecessor IS NULL OR reason IS NULL)) THEN
        RAISE EXCEPTION 'tracked deadline action disagrees with predecessor or reason' USING ERRCODE='23514';
    END IF;
    IF (author_tag=0 AND (action=4 OR cause IS NOT NULL))
        OR (author_tag=1 AND (action<>4 OR cause IS NULL)) THEN
        RAISE EXCEPTION 'tracked deadline action or cause disagrees with author' USING ERRCODE='23514';
    END IF;
    RETURN jsonb_build_object('version',2,'case_id',receipt_case,
        'deadline_id',encode(substring(b FROM 22 FOR 16),'hex')::uuid,
        'operation_id',encode(substring(b FROM 38 FOR 16),'hex')::uuid,
        'action',(ARRAY['register','correct','set_attention','retire','reevaluate'])[action+1],
        'expected_revision',revision,'review_digest',encode(substring(b FROM 59 FOR 32),'hex'),
        'observations_digest',encode(substring(b FROM 91 FOR 32),'hex'),
        'predecessor',predecessor,'author',author,'reason',reason,'cause',cause);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid tracked deadline receipt bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_submission_v2(BYTEA) FROM PUBLIC;
