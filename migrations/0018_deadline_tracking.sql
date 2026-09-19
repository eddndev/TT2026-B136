-- Decode the existing DLST2 tracking suffix without changing historical calculation bytes.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_tracking(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE size INTEGER; policies INTEGER[]; state INTEGER; reason_count INTEGER;
    p INTEGER:=5; reason_index INTEGER; dependency INTEGER; reason INTEGER;
    reason_key INTEGER; previous_key INTEGER:=-1; admin_tag INTEGER;
    administration_revision BIGINT; profile_policy_reason BOOLEAN:=FALSE;
    observations_digest TEXT; reasons JSONB:='[]'::jsonb;
BEGIN
    IF b IS NULL THEN RETURN NULL; END IF;
    size:=octet_length(b);
    IF size NOT BETWEEN 102 AND 122 THEN
        RAISE EXCEPTION 'invalid deadline tracking suffix size' USING ERRCODE='23514';
    END IF;
    policies:=ARRAY[get_byte(b,0),get_byte(b,1),get_byte(b,2)];
    state:=get_byte(b,3);reason_count:=get_byte(b,4);
    IF policies[1] NOT BETWEEN 0 AND 2 OR policies[2] NOT BETWEEN 0 AND 2
        OR policies[3] NOT BETWEEN 0 AND 2 OR state NOT BETWEEN 0 AND 2
        OR reason_count>8 THEN
        RAISE EXCEPTION 'invalid deadline tracking tags or reason count' USING ERRCODE='23514';
    END IF;
    IF (state IN (0,1) AND reason_count<>0) OR (state=2 AND reason_count=0) THEN
        RAISE EXCEPTION 'deadline tracking review and reason presence disagree' USING ERRCODE='23514';
    END IF;
    IF (state=0 AND policies<>ARRAY[0,0,0]) OR (state=1 AND policies[1]=0) THEN
        RAISE EXCEPTION 'deadline tracking review and profile policy disagree' USING ERRCODE='23514';
    END IF;
    FOR reason_index IN 1..reason_count LOOP
        dependency:=get_byte(b,p);reason:=get_byte(b,p+1);p:=p+2;
        IF dependency NOT BETWEEN 0 AND 2 OR reason NOT BETWEEN 0 AND 3
            OR (reason=0 AND dependency<>1) OR (reason=1 AND dependency<>0) THEN
            RAISE EXCEPTION 'invalid deadline tracking dependency or reason' USING ERRCODE='23514';
        END IF;
        reason_key:=dependency*4+reason;
        IF reason_key<=previous_key THEN
            RAISE EXCEPTION 'deadline tracking reasons must be strictly ordered' USING ERRCODE='23514';
        END IF;
        previous_key:=reason_key;
        IF (reason IN (0,1) AND policies[dependency+1]<>2)
            OR (reason=3 AND policies[dependency+1]<>0) THEN
            RAISE EXCEPTION 'deadline tracking reason disagrees with its policy' USING ERRCODE='23514';
        END IF;
        IF dependency=0 AND reason=3 THEN profile_policy_reason:=TRUE; END IF;
        reasons:=reasons||jsonb_build_array(jsonb_build_object(
            'dependency',(ARRAY['profile','source','calendar'])[dependency+1],
            'reason',(ARRAY['source_changed','profile_changed','dependency_retired','policy_undetermined'])[reason+1]));
    END LOOP;
    IF state=2 AND policies[1]=0 AND NOT profile_policy_reason THEN
        RAISE EXCEPTION 'undeclared profile policy requires its pending review reason' USING ERRCODE='23514';
    END IF;
    observations_digest:=encode(substring(b FROM p+1 FOR 32),'hex');p:=p+32;
    admin_tag:=get_byte(b,p);p:=p+1;
    IF admin_tag NOT BETWEEN 0 AND 1 OR size<>102+2*reason_count+4*admin_tag THEN
        RAISE EXCEPTION 'invalid deadline tracking administration presence or length' USING ERRCODE='23514';
    END IF;
    IF admin_tag=1 THEN
        administration_revision:=%1$I.typed_u32(b,p);p:=p+4;
        IF administration_revision=0 THEN
            RAISE EXCEPTION 'zero deadline tracking administration revision' USING ERRCODE='23514';
        END IF;
    END IF;
    -- Source and calendar presence belongs to the accompanying DLOB1 manifest.
    RETURN jsonb_build_object(
        'policies',jsonb_build_object(
            'profile',(ARRAY['undetermined','fixed','follow'])[policies[1]+1],
            'source',(ARRAY['undetermined','fixed','follow'])[policies[2]+1],
            'calendar',(ARRAY['undetermined','fixed','follow'])[policies[3]+1]),
        'review',jsonb_build_object('state',(ARRAY['legacy_undeclared','accepted','pending'])[state+1],
            'reasons',reasons),
        'observations_digest',observations_digest,'administration_revision',administration_revision,
        'administration_values_digest',encode(substring(b FROM p+1 FOR 32),'hex'),
        'administration_evidence_digest',encode(substring(b FROM p+33 FOR 32),'hex'));
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation THEN
    RAISE EXCEPTION 'invalid deadline tracking suffix bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_tracking(BYTEA) FROM PUBLIC;
