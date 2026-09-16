-- Decoded values are projections of immutable bounded canonical bytes.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_subject_values(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE p INTEGER:=6;tag INTEGER;value JSONB;nested JSONB;name JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 7 AND 5676 OR substring(b FROM 1 FOR 5)<>convert_to('SUBJ1','UTF8') THEN
        RAISE EXCEPTION 'invalid SUBJ1 bytes' USING ERRCODE='23514';
    END IF;
    tag:=get_byte(b,5);
    IF tag=0 THEN
        value:=jsonb_build_object('kind','natural_person');
        tag:=get_byte(b,p);p:=p+1;
        nested:=%1$I.typed_read(b,p,'text200');p:=(nested->>'next')::integer;
        IF tag=0 THEN name:=jsonb_build_object('known',nested->'value');
        ELSIF tag=1 THEN
            name:=jsonb_build_object('label',nested->'value');
            nested:=%1$I.typed_read(b,p,'text500');p:=(nested->>'next')::integer;
            name:=name||jsonb_build_object('reason',nested->'value');
        ELSE RAISE EXCEPTION 'invalid represented name' USING ERRCODE='23514'; END IF;
        nested:=%1$I.typed_read(b,p,'declared_curp');p:=(nested->>'next')::integer;
        value:=value||jsonb_build_object('name',name,'curp',nested->'value');
    ELSIF tag=1 THEN
        nested:=%1$I.typed_read(b,p,'text200');p:=(nested->>'next')::integer;
        value:=jsonb_build_object('kind','institutional_body','name',nested->'value');
        nested:=%1$I.typed_read(b,p,'declared_text80');p:=(nested->>'next')::integer;
        value:=value||jsonb_build_object('institutional_identifier',nested->'value');
    ELSE RAISE EXCEPTION 'invalid represented subject kind' USING ERRCODE='23514'; END IF;
    nested:=%1$I.typed_read(b,p,'locator');p:=(nested->>'next')::integer;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing SUBJ1 bytes' USING ERRCODE='23514'; END IF;
    RETURN value||jsonb_build_object('identity_support',nested->'value');
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_participant_values(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE p INTEGER:=58;tag INTEGER;revision BIGINT;value JSONB;nested JSONB;profile JSONB;spec TEXT;specs TEXT[];
    names TEXT[]:=ARRAY['defendant','victim','defense_counsel','prosecutor','victim_counsel','control_judge','trial_court','expert','police','precautionary_supervisor','other'];
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 62 AND 8380 OR substring(b FROM 1 FOR 5)<>convert_to('PART2','UTF8') THEN
        RAISE EXCEPTION 'invalid PART2 bytes' USING ERRCODE='23514';
    END IF;
    revision:=%1$I.typed_u32(b,21);
    IF revision=0 THEN RAISE EXCEPTION 'invalid bound subject revision' USING ERRCODE='23514'; END IF;
    value:=jsonb_build_object('subject',jsonb_build_object('id',encode(substring(b FROM 6 FOR 16),'hex')::uuid,
        'revision',revision,'digest',encode(substring(b FROM 26 FOR 32),'hex')));
    tag:=get_byte(b,57);
    IF tag NOT IN (0,1) THEN RAISE EXCEPTION 'invalid directory status' USING ERRCODE='23514'; END IF;
    value:=value||jsonb_build_object('directory_status',CASE tag WHEN 0 THEN 'active' ELSE 'archived' END);
    nested:=%1$I.typed_read(b,p,'optional_text200');p:=(nested->>'next')::integer;
    value:=value||jsonb_build_object('organization',nested->'value');
    nested:=%1$I.typed_read(b,p,'optional_text160');p:=(nested->>'next')::integer;
    value:=value||jsonb_build_object('legal_status',nested->'value');
    tag:=get_byte(b,p);p:=p+1;
    IF tag NOT BETWEEN 0 AND 10 THEN RAISE EXCEPTION 'invalid participant kind' USING ERRCODE='23514'; END IF;
    profile:=jsonb_build_object('kind',names[tag+1]);
    specs:=CASE tag
        WHEN 0 THEN ARRAY['custody:declared_custody']
        WHEN 1 THEN ARRAY['contact:contact','protection:protection']
        WHEN 2 THEN ARRAY['license:license','mode:mode']
        WHEN 3 THEN ARRAY['office_identifier:declared_text80','unit:declared_text200','license:declared_license']
        WHEN 4 THEN ARRAY['institution:text200','license:declared_license']
        WHEN 5 THEN ARRAY['court:text200']
        WHEN 6 THEN ARRAY['judicial_district:text200','composition:composition']
        WHEN 7 THEN ARRAY['specialty:declared_text200','license:declared_license']
        WHEN 8 THEN ARRAY['agency:declared_text200','unit:declared_text200']
        WHEN 9 THEN ARRAY['authority:declared_text200','unit:declared_text200']
        ELSE ARRAY['label:text80','description:declared_text200'] END;
    FOREACH spec IN ARRAY specs LOOP
        nested:=%1$I.typed_read(b,p,split_part(spec,':',2));p:=(nested->>'next')::integer;
        profile:=profile||jsonb_build_object(split_part(spec,':',1),nested->'value');
    END LOOP;
    nested:=%1$I.typed_read(b,p,'locator');p:=(nested->>'next')::integer;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing PART2 bytes' USING ERRCODE='23514'; END IF;
    RETURN value||jsonb_build_object('profile',profile,'role_support',nested->'value');
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION typed_subject_values(BYTEA),typed_participant_values(BYTEA) FROM PUBLIC;
