-- HRES1 is decoded independently from Rust, including ordering and scalar bounds.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.hearing_result_values(b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE p INTEGER:=7; occurrence INTEGER; extent INTEGER; n INTEGER; tag INTEGER;
    item JSONB; result JSONB; attendees JSONB:='[]'; agreements JSONB:='[]'; provenance JSONB;
    id UUID; previous UUID; ids UUID[]:='{}'; revision BIGINT; capacity JSONB; observation JSONB;
    kind INTEGER; reference JSONB; support JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 26 AND 146933
        OR substring(b FROM 1 FOR 5)<>convert_to('HRES1','UTF8') THEN
        RAISE EXCEPTION 'invalid result canonical header' USING ERRCODE='23514';
    END IF;
    occurrence:=get_byte(b,5);extent:=get_byte(b,6);
    IF occurrence NOT IN (0,1) OR extent NOT BETWEEN 0 AND 2 OR (occurrence=1 AND extent<>2) THEN
        RAISE EXCEPTION 'invalid result occurrence or extent' USING ERRCODE='23514';
    END IF;
    result:=jsonb_build_object('occurrence',CASE occurrence WHEN 0 THEN 'occurred' ELSE 'not_started' END,
        'extent',(ARRAY['partial','concluded','unspecified'])[extent+1]);
    item:=%1$I.hearing_result_time(b,p);p:=(item->>'next')::integer;
    result:=result||jsonb_build_object('event_time',item->'value');
    item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;
    result:=result||jsonb_build_object('summary',item->'value');
    n:=get_byte(b,p);p:=p+1;
    IF n>32 THEN RAISE EXCEPTION 'too many result attendees' USING ERRCODE='23514'; END IF;
    FOR i IN 1..n LOOP
        IF p+20>octet_length(b) THEN RAISE EXCEPTION 'truncated result attendee' USING ERRCODE='23514'; END IF;
        id:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;
        revision:=%1$I.typed_u32(b,p+16);p:=p+20;
        IF revision=0 OR (previous IS NOT NULL AND id<=previous) THEN
            RAISE EXCEPTION 'result attendees must be exact and strictly ordered' USING ERRCODE='23514';
        END IF;
        item:=%1$I.hearing_text(b,p,100,FALSE);p:=(item->>'next')::integer;capacity:=item->'value';
        tag:=get_byte(b,p);p:=p+1;
        IF tag=0 THEN observation:=NULL;
        ELSIF tag=1 THEN
            item:=%1$I.hearing_text(b,p,500,TRUE);p:=(item->>'next')::integer;observation:=item->'value';
        ELSE RAISE EXCEPTION 'invalid result observation tag' USING ERRCODE='23514'; END IF;
        attendees:=attendees||jsonb_build_array(jsonb_build_object('participant_id',id,'revision',revision,
            'capacity',capacity,'observation',observation));
        previous:=id;
    END LOOP;
    n:=get_byte(b,p);p:=p+1;
    IF n>16 THEN RAISE EXCEPTION 'too many result agreements' USING ERRCODE='23514'; END IF;
    FOR i IN 1..n LOOP
        IF p+16>octet_length(b) THEN RAISE EXCEPTION 'truncated result agreement' USING ERRCODE='23514'; END IF;
        id:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
        IF id=ANY(ids) THEN RAISE EXCEPTION 'duplicate result agreement' USING ERRCODE='23514'; END IF;
        ids:=array_append(ids,id);
        item:=%1$I.hearing_text(b,p,1000,TRUE);p:=(item->>'next')::integer;
        agreements:=agreements||jsonb_build_array(jsonb_build_object('id',id,'text',item->'value'));
    END LOOP;
    kind:=get_byte(b,p);p:=p+1;
    IF kind NOT BETWEEN 0 AND 2 THEN RAISE EXCEPTION 'invalid result provenance' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 AND kind=0 THEN reference:=NULL;
    ELSIF tag=1 THEN
        item:=%1$I.hearing_text(b,p,200,FALSE);p:=(item->>'next')::integer;reference:=item->'value';
    ELSE RAISE EXCEPTION 'result provenance reference is absent or invalid' USING ERRCODE='23514'; END IF;
    tag:=get_byte(b,p);p:=p+1;
    IF tag=0 THEN support:=NULL;
    ELSIF tag=1 THEN
        IF p+52>octet_length(b) THEN RAISE EXCEPTION 'truncated result support' USING ERRCODE='23514'; END IF;
        revision:=%1$I.typed_u32(b,p+16);
        IF revision=0 THEN RAISE EXCEPTION 'invalid result support version' USING ERRCODE='23514'; END IF;
        support:=jsonb_build_object('document_id',encode(substring(b FROM p+1 FOR 16),'hex')::uuid,
            'version',revision,'digest',encode(substring(b FROM p+21 FOR 32),'hex'));p:=p+52;
    ELSE RAISE EXCEPTION 'invalid result support tag' USING ERRCODE='23514'; END IF;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing result canonical bytes' USING ERRCODE='23514'; END IF;
    provenance:=jsonb_build_object('kind',(ARRAY['operator_note','oral_reference','written_record'])[kind+1],
        'reference',reference,'support',support);
    RETURN result||jsonb_build_object('attendees',attendees,'agreements',agreements,'provenance',provenance);
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character THEN
    RAISE EXCEPTION 'invalid result canonical bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION hearing_result_values(BYTEA) FROM PUBLIC;
