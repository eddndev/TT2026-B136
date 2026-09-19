-- Observe real current heads while retaining each selected historical dependency.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.deadline_worker_observations(
    base %1$I.case_deadline_revisions, bytes BYTEA)
RETURNS JSONB LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE manifest JSONB; old_manifest JSONB; entry JSONB; old_entry JSONB; captured RECORD;
    role TEXT; observed_family TEXT; identifier UUID; selected BIGINT; minimum_revision BIGINT;
    previous_revision BIGINT; expected_case UUID; expected_hearing UUID;
    result JSONB:='[]'::jsonb; expected_count INTEGER;
BEGIN
    manifest:=%1$I.deadline_observations(bytes);
    IF (manifest->>'case_id')::uuid IS DISTINCT FROM base.case_id THEN
        RAISE EXCEPTION 'deadline worker observations belong to another case' USING ERRCODE='23514'; END IF;
    IF base.observations_canonical IS NOT NULL THEN
        old_manifest:=%1$I.deadline_observations(base.observations_canonical);
    END IF;
    expected_count:=1+(base.source_id IS NOT NULL)::integer+(base.calendar_id IS NOT NULL)::integer
        +coalesce((base.source_kind='notification')::integer,0);
    IF jsonb_array_length(manifest->'entries') IS DISTINCT FROM expected_count THEN
        RAISE EXCEPTION 'deadline worker observation presence differs from selections' USING ERRCODE='23514'; END IF;
    FOR entry IN SELECT jsonb_array_elements(manifest->'entries') LOOP
        role:=entry->>'role';observed_family:=entry->>'family';
        expected_case:=base.case_id;expected_hearing:=NULL;
        CASE role
        WHEN 'profile' THEN
            identifier:=base.profile_id;selected:=base.profile_revision;minimum_revision:=selected;
            SELECT case_id INTO expected_case FROM %1$I.deadline_profiles WHERE id=identifier;
        WHEN 'source' THEN
            identifier:=base.source_id;selected:=base.source_revision;minimum_revision:=base.source_head_revision;
            expected_hearing:=base.source_hearing_id;
            IF observed_family IS DISTINCT FROM base.source_kind THEN
                RAISE EXCEPTION 'deadline worker source family differs' USING ERRCODE='23514'; END IF;
        WHEN 'calendar' THEN
            identifier:=base.calendar_id;selected:=base.calendar_revision;minimum_revision:=base.calendar_head_revision;
            expected_case:=NULL;
        WHEN 'notification_parent' THEN
            identifier:=base.source_parent_resolution_id;selected:=base.source_parent_resolution_revision;
            minimum_revision:=greatest(selected,base.source_head_parent_resolution_revision);
            IF base.source_kind IS DISTINCT FROM 'notification' THEN
                RAISE EXCEPTION 'deadline worker parent has no selected notification' USING ERRCODE='23514'; END IF;
        END CASE;
        IF identifier IS NULL OR (entry->>'id')::uuid IS DISTINCT FROM identifier
            OR (entry->>'revision')::bigint<minimum_revision
            OR (entry->>'case_id')::uuid IS DISTINCT FROM expected_case
            OR (entry->>'hearing_id')::uuid IS DISTINCT FROM expected_hearing THEN
            RAISE EXCEPTION 'deadline worker observed selection or scope differs' USING ERRCODE='23514'; END IF;
        CASE observed_family
        WHEN 'profile' THEN
            SELECT r.revision,r.status,r.submission_digest,p.case_id,NULL::uuid AS hearing_id,
                NULL::uuid AS parent_id,NULL::bigint AS parent_revision INTO captured
                FROM %1$I.deadline_profile_revisions r JOIN %1$I.deadline_profiles p ON p.id=r.profile_id
                WHERE r.profile_id=identifier ORDER BY r.revision DESC LIMIT 1;
        WHEN 'resolution','notification' THEN
            SELECT revision,status,submission_digest,case_id,NULL::uuid AS hearing_id,
                (values_view->'resolution'->>'id')::uuid AS parent_id,
                (values_view->'resolution'->>'revision')::bigint AS parent_revision INTO captured
                FROM %1$I.case_procedural_fact_revisions
                WHERE family=observed_family AND id=identifier ORDER BY revision DESC LIMIT 1;
        WHEN 'hearing_result' THEN
            SELECT revision,status,submission_digest,case_id,hearing_id,
                NULL::uuid AS parent_id,NULL::bigint AS parent_revision INTO captured
                FROM %1$I.case_hearing_result_revisions WHERE result_id=identifier ORDER BY revision DESC LIMIT 1;
        WHEN 'calendar' THEN
            SELECT revision,status,submission_digest,NULL::uuid AS case_id,NULL::uuid AS hearing_id,
                NULL::uuid AS parent_id,NULL::bigint AS parent_revision INTO captured
                FROM %1$I.judicial_calendar_revisions WHERE calendar_id=identifier ORDER BY revision DESC LIMIT 1;
        END CASE;
        IF captured.revision IS NULL OR captured.revision IS DISTINCT FROM (entry->>'revision')::bigint
            OR captured.case_id IS DISTINCT FROM expected_case OR captured.hearing_id IS DISTINCT FROM expected_hearing
            OR captured.submission_digest IS DISTINCT FROM decode(entry->>'submission_digest','hex')
            OR captured.parent_id IS DISTINCT FROM (entry->'parent_resolution'->>'id')::uuid
            OR captured.parent_revision IS DISTINCT FROM (entry->'parent_resolution'->>'revision')::bigint
            OR (observed_family='notification' AND captured.parent_id IS DISTINCT FROM base.source_parent_resolution_id) THEN
            RAISE EXCEPTION 'deadline worker observation is not its exact current head' USING ERRCODE='23514'; END IF;
        SELECT item INTO old_entry FROM jsonb_array_elements(old_manifest->'entries') item WHERE item->>'role'=role;
        previous_revision:=(old_entry->>'revision')::bigint;
        IF old_entry IS NOT NULL THEN
            IF old_entry->>'family' IS DISTINCT FROM entry->>'family'
                OR old_entry->>'id' IS DISTINCT FROM entry->>'id'
                OR old_entry->'case_id' IS DISTINCT FROM entry->'case_id'
                OR old_entry->'hearing_id' IS DISTINCT FROM entry->'hearing_id'
                OR old_entry->'parent_resolution'->>'id' IS DISTINCT FROM entry->'parent_resolution'->>'id'
                OR previous_revision>captured.revision
                OR (previous_revision=captured.revision AND old_entry IS DISTINCT FROM entry) THEN
                RAISE EXCEPTION 'deadline worker observation regresses or replaces old evidence' USING ERRCODE='23514'; END IF;
        ELSIF old_manifest IS NULL AND role<>'notification_parent' THEN
            previous_revision:=minimum_revision;
        END IF;
        result:=result||jsonb_build_array(entry||jsonb_build_object('status',captured.status,
            'selected_revision',selected,'prior_revision',previous_revision,
            'observed_revision',coalesce(previous_revision,selected)));
    END LOOP;
    IF old_manifest IS NOT NULL AND EXISTS(SELECT 1 FROM jsonb_array_elements(old_manifest->'entries') old
        WHERE NOT EXISTS(SELECT 1 FROM jsonb_array_elements(result) item WHERE item->>'role'=old->>'role')) THEN
        RAISE EXCEPTION 'deadline worker discarded a prior observation' USING ERRCODE='23514'; END IF;
    RETURN result;
END; $$;
CREATE OR REPLACE FUNCTION %1$I.deadline_worker_administration(
    base %1$I.case_deadline_revisions, revision BIGINT, evidence_digest BYTEA)
RETURNS BYTEA LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE baseline RECORD; captured RECORD; current_revision BIGINT; old_tracking JSONB;
    previous_revision BIGINT; values_bytes BYTEA; values_digest BYTEA;
BEGIN
    SELECT * INTO baseline FROM %1$I.cases WHERE id=base.case_id;
    SELECT max(r.revision) INTO current_revision FROM %1$I.case_administration_revisions r WHERE case_id=base.case_id;
    previous_revision:=base.observed_administration_revision;
    IF base.tracking_canonical IS NOT NULL THEN
        old_tracking:=%1$I.deadline_tracking(base.tracking_canonical);
        previous_revision:=(old_tracking->>'administration_revision')::bigint;
    END IF;
    IF baseline.id IS NULL OR revision IS DISTINCT FROM current_revision
        OR (previous_revision IS NOT NULL AND (revision IS NULL OR revision<previous_revision))
        OR octet_length(evidence_digest) IS DISTINCT FROM 32 THEN
        RAISE EXCEPTION 'deadline worker administration is absent, stale or regresses' USING ERRCODE='23514'; END IF;
    IF revision IS NULL THEN
        IF baseline.required_initial_revision IS NOT NULL THEN
            RAISE EXCEPTION 'deadline worker baseline has a required revision' USING ERRCODE='23514'; END IF;
        values_bytes:=%1$I.case_administration_bytes('active',baseline.title,baseline.reference,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
        values_digest:=sha256(values_bytes);
        IF evidence_digest IS DISTINCT FROM sha256(values_digest||decode('00','hex')) THEN
            RAISE EXCEPTION 'deadline worker baseline evidence differs' USING ERRCODE='23514'; END IF;
    ELSE
        SELECT * INTO captured FROM %1$I.case_administration_revisions r
            WHERE case_id=base.case_id AND r.revision=deadline_worker_administration.revision;
        values_bytes:=%1$I.case_administration_bytes(captured.administrative_status,captured.title,captured.reference,
            captured.nuc,captured.nuc_authority,captured.judicial_case_number,captured.judicial_authority,
            captured.offenses,captured.general_information,captured.complementary_identifiers);
        values_digest:=sha256(values_bytes);
        IF captured.revision IS NULL OR captured.values_digest IS DISTINCT FROM values_digest THEN
            RAISE EXCEPTION 'deadline worker administrative values differ' USING ERRCODE='23514'; END IF;
    END IF;
    IF old_tracking IS NOT NULL AND revision IS NOT DISTINCT FROM previous_revision
        AND evidence_digest IS DISTINCT FROM decode(old_tracking->>'administration_evidence_digest','hex') THEN
        RAISE EXCEPTION 'deadline worker changed immutable administrative evidence' USING ERRCODE='23514'; END IF;
    RETURN values_digest;
END; $$;
$definition$,current_schema());
END; $install$;
REVOKE ALL ON FUNCTION deadline_worker_observations(case_deadline_revisions,BYTEA),
    deadline_worker_administration(case_deadline_revisions,BIGINT,BYTEA) FROM PUBLIC;
