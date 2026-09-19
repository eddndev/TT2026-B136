-- Bounds and missing-job filtering apply before the returned page is materialized.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_dispatch_candidates(
    selected_event BIGINT,lower_id UUID,lower_inclusive BOOLEAN,upper_id UUID,
    missing_only BOOLEAN,page_limit INTEGER)
RETURNS TABLE(deadline_id UUID,case_id UUID)
LANGUAGE plpgsql STABLE SECURITY INVOKER SET search_path=pg_catalog AS $$
BEGIN
    IF lower_inclusive IS NULL OR missing_only IS NULL OR page_limit IS NULL
        OR page_limit NOT BETWEEN 1 AND 101 OR selected_event<=0
        OR (lower_id IS NOT NULL AND upper_id IS NOT NULL AND lower_id>upper_id) THEN
        RAISE EXCEPTION 'invalid deadline dispatch candidate bounds' USING ERRCODE='23514';
    END IF;
    RETURN QUERY
    SELECT d.id,d.case_id FROM %1$I.case_deadlines d
    JOIN LATERAL (
        SELECT r.status,r.source_kind,r.source_id,r.source_hearing_id,
            r.source_parent_resolution_id,r.calendar_id,r.profile_id,r.tracking_canonical
        FROM %1$I.case_deadline_revisions r WHERE r.deadline_id=d.id
        ORDER BY r.revision DESC LIMIT 1
    ) h ON TRUE
    LEFT JOIN %1$I.deadline_source_events e ON e.sequence=selected_event
    -- UUID extrema retain indexable bounds while distinguishing nil from no cursor.
    WHERE d.id>=coalesce(lower_id,'00000000-0000-0000-0000-000000000000'::uuid)
        AND d.id<=coalesce(upper_id,'ffffffff-ffff-ffff-ffff-ffffffffffff'::uuid)
        AND (lower_id IS NULL OR lower_inclusive OR d.id>lower_id)
        AND h.status='active' AND (
            (selected_event IS NULL AND (h.tracking_canonical IS NULL OR
                %1$I.deadline_tracking(h.tracking_canonical)#>>'{review,state}'='legacy_undeclared'))
            OR (selected_event IS NOT NULL AND CASE e.source_kind
                WHEN 'resolution' THEN e.case_id=d.case_id AND (
                    (h.source_kind='resolution' AND h.source_id=e.source_id)
                    OR (h.source_kind='notification' AND h.source_parent_resolution_id=e.source_id))
                WHEN 'notification' THEN e.case_id=d.case_id
                    AND h.source_kind='notification' AND h.source_id=e.source_id
                WHEN 'hearing_result' THEN e.case_id=d.case_id
                    AND h.source_kind='hearing_result' AND h.source_id=e.source_id
                    AND h.source_hearing_id=e.hearing_id
                WHEN 'calendar' THEN h.calendar_id=e.source_id
                WHEN 'profile' THEN h.profile_id=e.source_id
                    AND (e.case_id IS NULL OR e.case_id=d.case_id)
                ELSE FALSE END))
        AND (NOT missing_only OR NOT EXISTS(
            SELECT 1 FROM %1$I.deadline_reevaluation_jobs j WHERE j.deadline_id=d.id
                AND ((selected_event IS NOT NULL AND j.event_sequence=selected_event)
                    OR (selected_event IS NULL AND j.event_sequence IS NULL
                        AND j.bootstrap_policy_version=1))))
    ORDER BY d.id LIMIT page_limit;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_dispatch_candidates(BIGINT,UUID,BOOLEAN,UUID,BOOLEAN,INTEGER) FROM PUBLIC;
