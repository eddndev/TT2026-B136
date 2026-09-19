-- Check manifest commitments and declared tracking semantics, not reference authenticity.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_tracking_consistent(tracking_bytes BYTEA, observation_bytes BYTEA)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE tracking JSONB; observations JSONB; entry JSONB; requirement JSONB;
    state TEXT; dependency TEXT; policy TEXT; present BOOLEAN;
    source_present BOOLEAN:=FALSE; calendar_present BOOLEAN:=FALSE;
    notification_source BOOLEAN:=FALSE; parent_present BOOLEAN:=FALSE;
    has_reasons BOOLEAN; has_policy_reason BOOLEAN;
BEGIN
    IF tracking_bytes IS NULL AND observation_bytes IS NULL THEN RETURN TRUE; END IF;
    IF tracking_bytes IS NULL OR observation_bytes IS NULL THEN RETURN FALSE; END IF;
    -- Parse both payloads before comparing them so invalid bytes never become FALSE.
    tracking:=%1$I.deadline_tracking(tracking_bytes);
    observations:=%1$I.deadline_observations(observation_bytes);
    IF tracking->>'observations_digest'<>encode(pg_catalog.sha256(observation_bytes),'hex') THEN
        RETURN FALSE;
    END IF;
    state:=tracking->'review'->>'state';
    FOR entry IN SELECT jsonb_array_elements(observations->'entries') LOOP
        IF entry->>'role'='source' THEN
            source_present:=TRUE;
            notification_source:=entry->>'family'='notification';
        ELSIF entry->>'role'='calendar' THEN
            calendar_present:=TRUE;
        ELSIF entry->>'role'='notification_parent' THEN
            parent_present:=TRUE;
        END IF;
    END LOOP;
    FOREACH dependency IN ARRAY ARRAY['profile','source','calendar'] LOOP
        policy:=tracking->'policies'->>dependency;
        present:=CASE dependency WHEN 'profile' THEN TRUE
            WHEN 'source' THEN source_present ELSE calendar_present END;
        has_reasons:=FALSE;has_policy_reason:=FALSE;
        FOR requirement IN SELECT jsonb_array_elements(tracking->'review'->'reasons') LOOP
            IF requirement->>'dependency'=dependency THEN
                has_reasons:=TRUE;
                IF requirement->>'reason'='policy_undetermined' THEN has_policy_reason:=TRUE; END IF;
            END IF;
        END LOOP;
        IF NOT present THEN
            IF policy<>'undetermined' OR has_reasons THEN RETURN FALSE; END IF;
        ELSIF state='accepted' AND policy='undetermined' THEN
            RETURN FALSE;
        ELSIF state='pending' AND policy='undetermined' AND NOT has_policy_reason THEN
            RETURN FALSE;
        END IF;
    END LOOP;
    IF state='accepted' AND notification_source AND NOT parent_present THEN RETURN FALSE; END IF;
    RETURN TRUE;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_tracking_consistent(BYTEA,BYTEA) FROM PUBLIC;
