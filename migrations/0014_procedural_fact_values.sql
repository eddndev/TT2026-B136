-- PFRES1 and PFNOT1 preserve declarations without inferring legal effects.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.procedural_fact_values(family TEXT,b BYTEA)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$
DECLARE p INTEGER:=6; value JSONB:='{}'; item JSONB; field TEXT; reference JSONB:='{}';
    tag INTEGER; effect JSONB; first_support JSONB; second_support JSONB;
BEGIN
    IF family IS NULL OR family NOT IN ('resolution','notification') OR b IS NULL
        OR (family='resolution' AND (octet_length(b) NOT BETWEEN 27 AND 17700
            OR substring(b FROM 1 FOR 6)<>convert_to('PFRES1','UTF8')))
        OR (family='notification' AND (octet_length(b) NOT BETWEEN 67 AND 58671
            OR substring(b FROM 1 FOR 6)<>convert_to('PFNOT1','UTF8'))) THEN
        RAISE EXCEPTION 'invalid procedural fact values header' USING ERRCODE='23514'; END IF;
    IF family='resolution' THEN
        FOREACH field IN ARRAY ARRAY['class','subtype','issuer','issued_at','summary','provenance'] LOOP
            IF field IN ('class','issuer') THEN item:=%1$I.procedural_fact_declaration(b,p,field);
            ELSIF field='subtype' THEN item:=%1$I.procedural_fact_atom(b,p,'optional_label');
            ELSIF field='issued_at' THEN item:=%1$I.procedural_fact_time(b,p,'fact');
            ELSIF field='summary' THEN item:=%1$I.procedural_fact_atom(b,p,'text');
            ELSE item:=%1$I.procedural_fact_provenance(b,p); END IF;
            p:=(item->>'next')::integer;value:=value||jsonb_build_object(field,item->'value');
        END LOOP;
    ELSE
        FOREACH field IN ARRAY ARRAY['id','revision'] LOOP
            item:=%1$I.procedural_fact_atom(b,p,CASE field WHEN 'id' THEN 'uuid' ELSE 'revision' END);
            p:=(item->>'next')::integer;reference:=reference||jsonb_build_object(field,item->'value');
        END LOOP;
        value:=jsonb_build_object('resolution',reference);
        FOREACH field IN ARRAY ARRAY['character','medium','context','outcome'] LOOP
            item:=%1$I.procedural_fact_declaration(b,p,field);p:=(item->>'next')::integer;
            value:=value||jsonb_build_object(field,item->'value');
        END LOOP;
        item:=%1$I.procedural_fact_atom(b,p,'optional_label');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('subtype',item->'value');
        item:=%1$I.procedural_fact_time(b,p,'fact');p:=(item->>'next')::integer;
        value:=value||jsonb_build_object('practiced_at',item->'value');
        FOREACH field IN ARRAY ARRAY['received_at','stated_effect'] LOOP
            tag:=get_byte(b,p);p:=p+1;effect:=NULL;
            IF tag=1 THEN
                item:=%1$I.procedural_fact_time(b,p,'fact');p:=(item->>'next')::integer;effect:=item->'value';
                IF field='stated_effect' THEN
                    effect:=jsonb_build_object('at',effect);
                    item:=%1$I.procedural_fact_atom(b,p,'text');p:=(item->>'next')::integer;
                    effect:=effect||jsonb_build_object('statement',item->'value');
                    item:=%1$I.procedural_fact_atom(b,p,'label');p:=(item->>'next')::integer;
                    effect:=effect||jsonb_build_object('locator',item->'value');
                END IF;
            ELSIF tag<>0 THEN RAISE EXCEPTION 'invalid procedural fact time option' USING ERRCODE='23514'; END IF;
            value:=value||jsonb_build_object(field,effect);
        END LOOP;
        FOREACH field IN ARRAY ARRAY['intended_recipient','actual_receiver','representation','summary','provenance'] LOOP
            IF field IN ('intended_recipient','actual_receiver') THEN item:=%1$I.procedural_fact_declaration(b,p,'person');
            ELSIF field='representation' THEN item:=%1$I.procedural_fact_representation(b,p);
            ELSIF field='summary' THEN item:=%1$I.procedural_fact_atom(b,p,'text');
            ELSE item:=%1$I.procedural_fact_provenance(b,p); END IF;
            p:=(item->>'next')::integer;value:=value||jsonb_build_object(field,item->'value');
        END LOOP;
        first_support:=value#>'{provenance,support}';
        second_support:=value#>'{representation,provenance,support}';
        IF first_support->>'document_id'=second_support->>'document_id'
            AND first_support->>'version'=second_support->>'version'
            AND first_support->>'digest'<>second_support->>'digest' THEN
            RAISE EXCEPTION 'conflicting procedural fact support digests' USING ERRCODE='23514'; END IF;
    END IF;
    IF p<>octet_length(b) THEN
        RAISE EXCEPTION 'trailing procedural fact values bytes' USING ERRCODE='23514'; END IF;
    RETURN value;
EXCEPTION WHEN array_subscript_error OR numeric_value_out_of_range OR invalid_text_representation
    OR character_not_in_repertoire OR untranslatable_character OR datetime_field_overflow THEN
    RAISE EXCEPTION 'invalid procedural fact values bytes' USING ERRCODE='23514';
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION procedural_fact_values(TEXT,BYTEA) FROM PUBLIC;
