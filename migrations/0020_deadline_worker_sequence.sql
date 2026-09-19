-- Preserve the human path and admit only the separately guarded technical transition.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_deadline_sequence()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; actor RECORD; responsible RECORD; baseline RECORD; current_admin RECORD;
    captured RECORD; profile RECORD; selected_source JSONB; head_source JSONB; selection JSONB;
    admin_bytes BYTEA; latest BIGINT; parent UUID; receipt JSONB; tracking JSONB; observations JSONB;
    observation JSONB; tracked BOOLEAN; qualification BOOLEAN; legacy_upgrade BOOLEAN;
    outer_revision BIGINT; tracked_admin RECORD; tracked_admin_bytes BYTEA;
    selected_status TEXT; observed_role TEXT; observed_family TEXT; selected_id UUID;
    selected_revision BIGINT; expected_case UUID; expected_hearing UUID; expected_policy TEXT;
    source_present BOOLEAN:=FALSE; calendar_present BOOLEAN:=FALSE; parent_present BOOLEAN:=FALSE;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'deadline writes require read committed isolation' USING ERRCODE='23514'; END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    -- Generated NEW columns are unavailable to a BEFORE INSERT trigger.
    receipt:=%1$I.deadline_submission(NEW.submission_canonical);
    tracked:=substring(NEW.submission_canonical FROM 1 FOR 5)=convert_to('DLTX2','UTF8');
    qualification:=NEW.action IN ('register','correct');
    IF NEW.action='reevaluate' OR (tracked AND receipt->'author'->>'kind' IS DISTINCT FROM 'user') THEN
        PERFORM %1$I.enforce_deadline_technical(NEW);
        RETURN NEW;
    END IF;
    IF tracked THEN
        tracking:=%1$I.deadline_tracking(NEW.tracking_canonical);
        observations:=%1$I.deadline_observations(NEW.observations_canonical);
        IF %1$I.deadline_tracking_consistent(NEW.tracking_canonical,NEW.observations_canonical) IS DISTINCT FROM TRUE
            OR (observations->>'case_id')::uuid IS DISTINCT FROM NEW.case_id THEN
            RAISE EXCEPTION 'deadline tracking and observation captures disagree' USING ERRCODE='23514'; END IF;
    END IF;
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C"
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by))) THEN
        RAISE EXCEPTION 'deadline actor is not currently authorized' USING ERRCODE='42501'; END IF;
    SELECT * INTO baseline FROM %1$I.cases WHERE id=NEW.case_id;
    SELECT * INTO current_admin FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF baseline.id IS NULL OR (current_admin.revision IS NULL AND baseline.required_initial_revision IS NOT NULL)
        OR (current_admin.revision IS NOT NULL AND current_admin.administrative_status IS DISTINCT FROM 'active') THEN
        RAISE EXCEPTION 'deadline case must currently be active' USING ERRCODE='23514'; END IF;
    IF NEW.observed_administration_revision IS NULL THEN
        IF baseline.required_initial_revision IS NOT NULL THEN
            RAISE EXCEPTION 'deadline R0 requires an original unrevised baseline' USING ERRCODE='23514'; END IF;
        admin_bytes:=%1$I.case_administration_bytes('active',baseline.title,baseline.reference,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
    ELSE
        SELECT * INTO captured FROM %1$I.case_administration_revisions
            WHERE case_id=NEW.case_id AND revision=NEW.observed_administration_revision;
        IF captured.revision IS NULL THEN
            RAISE EXCEPTION 'deadline captured administration is absent' USING ERRCODE='23503'; END IF;
        admin_bytes:=%1$I.case_administration_bytes(captured.administrative_status,captured.title,captured.reference,
            captured.nuc,captured.nuc_authority,captured.judicial_case_number,captured.judicial_authority,
            captured.offenses,captured.general_information,captured.complementary_identifiers);
    END IF;
    IF admin_bytes IS DISTINCT FROM NEW.observed_administration_canonical THEN
        RAISE EXCEPTION 'deadline captured administration bytes differ' USING ERRCODE='23514'; END IF;
    SELECT * INTO previous FROM %1$I.case_deadline_revisions WHERE deadline_id=NEW.deadline_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM coalesce(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'register')
        OR (previous.revision IS NOT NULL AND (NEW.action='register' OR previous.status<>'active'
            OR previous.case_id IS DISTINCT FROM NEW.case_id)) THEN
        RAISE EXCEPTION 'deadline revisions require an active consecutive predecessor' USING ERRCODE='23514'; END IF;
    IF previous.revision IS NOT NULL THEN
        IF NOT tracked AND previous.tracking_canonical IS NOT NULL THEN
            RAISE EXCEPTION 'deadline history cannot discard V2 tracking' USING ERRCODE='23514'; END IF;
        IF tracked AND (decode(receipt->'predecessor'->>'submission_digest','hex') IS DISTINCT FROM previous.submission_digest
            OR decode(receipt->'predecessor'->>'capture_digest','hex') IS DISTINCT FROM previous.capture_digest) THEN
            RAISE EXCEPTION 'deadline predecessor receipts differ' USING ERRCODE='23514'; END IF;
    END IF;
    legacy_upgrade:=tracked AND previous.revision IS NOT NULL AND previous.tracking_canonical IS NULL;
    IF tracked THEN
        outer_revision:=(tracking->>'administration_revision')::bigint;
        IF outer_revision IS NULL THEN
            IF baseline.required_initial_revision IS NOT NULL THEN
                RAISE EXCEPTION 'tracked R0 requires an original unrevised baseline' USING ERRCODE='23514'; END IF;
            tracked_admin_bytes:=%1$I.case_administration_bytes('active',baseline.title,baseline.reference,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
        ELSE
            SELECT * INTO tracked_admin FROM %1$I.case_administration_revisions
                WHERE case_id=NEW.case_id AND revision=outer_revision;
            IF tracked_admin.revision IS NULL THEN
                RAISE EXCEPTION 'tracked administration revision is absent' USING ERRCODE='23503'; END IF;
            tracked_admin_bytes:=%1$I.case_administration_bytes(tracked_admin.administrative_status,tracked_admin.title,tracked_admin.reference,
                tracked_admin.nuc,tracked_admin.nuc_authority,tracked_admin.judicial_case_number,tracked_admin.judicial_authority,
                tracked_admin.offenses,tracked_admin.general_information,tracked_admin.complementary_identifiers);
        END IF;
        IF decode(tracking->>'administration_values_digest','hex') IS DISTINCT FROM sha256(tracked_admin_bytes) THEN
            RAISE EXCEPTION 'tracked administration values differ' USING ERRCODE='23514'; END IF;
        IF qualification AND (outer_revision IS DISTINCT FROM current_admin.revision
            OR tracking->'review'->>'state' IS DISTINCT FROM 'accepted') THEN
            RAISE EXCEPTION 'human qualification requires current accepted tracking' USING ERRCODE='23514'; END IF;
    END IF;
    SELECT p.case_id,r.revision,r.status INTO profile FROM %1$I.deadline_profiles p
        JOIN %1$I.deadline_profile_revisions r ON r.profile_id=p.id
        WHERE p.id=NEW.profile_id ORDER BY r.revision DESC LIMIT 1;
    IF profile.revision IS NULL OR (profile.case_id IS NOT NULL AND profile.case_id IS DISTINCT FROM NEW.case_id) THEN
        RAISE EXCEPTION 'deadline profile belongs to another case or is absent' USING ERRCODE='23514'; END IF;
    selection:=%1$I.deadline_input_selection(NEW.input_canonical);
    IF NEW.source_kind='notification' THEN
        SELECT parent_resolution_id INTO parent FROM %1$I.case_procedural_facts WHERE family='notification' AND id=NEW.source_id;
        SELECT values_view INTO selected_source FROM %1$I.case_procedural_fact_revisions
            WHERE family='notification' AND id=NEW.source_id AND revision=NEW.source_revision;
        SELECT values_view INTO head_source FROM %1$I.case_procedural_fact_revisions
            WHERE family='notification' AND id=NEW.source_id AND revision=NEW.source_head_revision;
        IF parent IS DISTINCT FROM NEW.source_parent_resolution_id
            OR (selected_source->'resolution'->>'id')::uuid IS DISTINCT FROM parent
            OR (head_source->'resolution'->>'id')::uuid IS DISTINCT FROM parent
            OR (selected_source->'resolution'->>'revision')::bigint IS DISTINCT FROM NEW.source_parent_resolution_revision
            OR (head_source->'resolution'->>'revision')::bigint IS DISTINCT FROM NEW.source_head_parent_resolution_revision THEN
            RAISE EXCEPTION 'deadline notification parent differs from selected or head revision' USING ERRCODE='23514'; END IF;
    ELSIF NEW.source_kind='hearing_result' AND selection->>'source_agreement_id' IS NOT NULL THEN
        IF NOT EXISTS(SELECT 1 FROM %1$I.case_hearing_result_revisions r,
            LATERAL jsonb_array_elements(r.values_view->'agreements') a
            WHERE r.result_id=NEW.source_id AND r.revision=NEW.source_revision
                AND a->>'id'=selection->>'source_agreement_id') THEN
            RAISE EXCEPTION 'deadline agreement is absent from the exact result' USING ERRCODE='23514'; END IF;
    END IF;
    IF qualification THEN
        IF NEW.observed_administration_revision IS DISTINCT FROM current_admin.revision
            OR (NOT tracked AND NEW.profile_revision IS DISTINCT FROM profile.revision) OR profile.status IS DISTINCT FROM 'published' THEN
            RAISE EXCEPTION 'deadline calculation requires current administration and published profile' USING ERRCODE='23514'; END IF;
        SELECT email,role,active INTO responsible FROM %1$I.users WHERE id=NEW.responsible_id FOR SHARE;
        IF responsible.active IS DISTINCT FROM TRUE
            OR responsible.role IS DISTINCT FROM NEW.responsible_role
            OR responsible.email COLLATE "C" IS DISTINCT FROM NEW.responsible_email COLLATE "C"
            OR NOT(responsible.role='owner' OR (responsible.role IN ('litigator','paralegal') AND EXISTS(
                SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.responsible_id))) THEN
            RAISE EXCEPTION 'deadline responsible is not currently eligible' USING ERRCODE='42501'; END IF;
        IF NEW.source_kind IN ('resolution','notification') THEN
            SELECT max(revision) INTO latest FROM %1$I.case_procedural_fact_revisions WHERE family=NEW.source_kind AND id=NEW.source_id;
        ELSIF NEW.source_kind='hearing_result' THEN
            SELECT max(revision) INTO latest FROM %1$I.case_hearing_result_revisions WHERE result_id=NEW.source_id;
        ELSE latest:=NULL; END IF;
        IF latest IS DISTINCT FROM NEW.source_head_revision THEN
            RAISE EXCEPTION 'deadline source head changed during calculation' USING ERRCODE='23514'; END IF;
        SELECT max(revision) INTO latest FROM %1$I.judicial_calendar_revisions WHERE calendar_id=NEW.calendar_id;
        IF latest IS DISTINCT FROM NEW.calendar_head_revision THEN
            RAISE EXCEPTION 'deadline calendar head changed during calculation' USING ERRCODE='23514'; END IF;
    ELSE
        IF ROW(NEW.title,NEW.profile_id,NEW.profile_revision,NEW.input_canonical,NEW.result_canonical,
            NEW.observed_administration_revision,NEW.observed_administration_canonical,NEW.observed_administration_digest,
            NEW.responsible_id,NEW.responsible_email,NEW.responsible_role,NEW.due_at_seconds,NEW.due_at_nanoseconds,
            NEW.source_kind,NEW.source_id,NEW.source_revision,NEW.source_head_revision,NEW.source_hearing_id,
            NEW.source_parent_resolution_id,NEW.source_parent_resolution_revision,NEW.source_head_parent_resolution_revision,
            NEW.calendar_id,NEW.calendar_revision,NEW.calendar_head_revision)
            IS DISTINCT FROM ROW(previous.title,previous.profile_id,previous.profile_revision,previous.input_canonical,previous.result_canonical,
            previous.observed_administration_revision,previous.observed_administration_canonical,previous.observed_administration_digest,
            previous.responsible_id,previous.responsible_email,previous.responsible_role,previous.due_at_seconds,previous.due_at_nanoseconds,
            previous.source_kind,previous.source_id,previous.source_revision,previous.source_head_revision,previous.source_hearing_id,
            previous.source_parent_resolution_id,previous.source_parent_resolution_revision,previous.source_head_parent_resolution_revision,
            previous.calendar_id,previous.calendar_revision,previous.calendar_head_revision) THEN
            RAISE EXCEPTION 'deadline attention and retirement preserve captured calculation' USING ERRCODE='23514'; END IF;
    END IF;
    IF tracked AND NOT qualification THEN
        IF previous.tracking_canonical IS NOT NULL THEN
            IF NEW.tracking_canonical IS DISTINCT FROM previous.tracking_canonical
                OR NEW.observations_canonical IS DISTINCT FROM previous.observations_canonical THEN
                RAISE EXCEPTION 'manual followup changed tracked observations or policies' USING ERRCODE='23514'; END IF;
        ELSIF tracking->'review'->>'state' IS DISTINCT FROM 'legacy_undeclared'
            OR outer_revision IS DISTINCT FROM NEW.observed_administration_revision
            OR decode(tracking->>'administration_values_digest','hex') IS DISTINCT FROM NEW.observed_administration_digest THEN
            RAISE EXCEPTION 'legacy followup must preserve undeclared historical tracking' USING ERRCODE='23514'; END IF;
    END IF;
    IF tracked THEN
        SELECT status INTO selected_status FROM %1$I.deadline_profile_revisions
            WHERE profile_id=NEW.profile_id AND revision=NEW.profile_revision;
        IF selected_status IS DISTINCT FROM 'published' THEN
            RAISE EXCEPTION 'captured profile revision must be published' USING ERRCODE='23514'; END IF;
        -- These checks bind references and receipts. The adapter reconstructs DLOE metadata.
        FOR observation IN SELECT jsonb_array_elements(observations->'entries') LOOP
            observed_role:=observation->>'role';observed_family:=observation->>'family';
            expected_case:=NEW.case_id;expected_hearing:=NULL;
            CASE observed_role
            WHEN 'profile' THEN
                selected_id:=NEW.profile_id;selected_revision:=NEW.profile_revision;expected_case:=profile.case_id;
                expected_policy:=tracking->'policies'->>'profile';
            WHEN 'source' THEN
                source_present:=TRUE;selected_id:=NEW.source_id;selected_revision:=NEW.source_revision;
                expected_hearing:=NEW.source_hearing_id;expected_policy:=tracking->'policies'->>'source';
                IF observed_family IS DISTINCT FROM NEW.source_kind THEN
                    RAISE EXCEPTION 'observed source family differs from the captured input' USING ERRCODE='23514'; END IF;
            WHEN 'calendar' THEN
                calendar_present:=TRUE;selected_id:=NEW.calendar_id;selected_revision:=NEW.calendar_revision;
                expected_case:=NULL;expected_policy:=tracking->'policies'->>'calendar';
            WHEN 'notification_parent' THEN
                parent_present:=TRUE;selected_id:=NEW.source_parent_resolution_id;
                selected_revision:=greatest(NEW.source_parent_resolution_revision,NEW.source_head_parent_resolution_revision);
                expected_policy:='fixed';
            END CASE;
            IF selected_id IS NULL OR (observation->>'id')::uuid IS DISTINCT FROM selected_id
                OR (observation->>'revision')::bigint<selected_revision
                OR (observation->>'case_id')::uuid IS DISTINCT FROM expected_case
                OR (observation->>'hearing_id')::uuid IS DISTINCT FROM expected_hearing THEN
                RAISE EXCEPTION 'observed dependency identity, scope or revision differs' USING ERRCODE='23514'; END IF;
            IF tracking->'review'->>'state'='accepted' AND expected_policy='follow'
                AND (observation->>'revision')::bigint IS DISTINCT FROM selected_revision THEN
                RAISE EXCEPTION 'accepted Follow dependency must retain its selected observed revision' USING ERRCODE='23514'; END IF;
            CASE observed_family
            WHEN 'profile' THEN
                SELECT r.revision,r.status,r.submission_digest,p.case_id,NULL::uuid AS hearing_id,
                    NULL::uuid AS parent_id,NULL::bigint AS parent_revision,
                    (SELECT max(revision) FROM %1$I.deadline_profile_revisions WHERE profile_id=r.profile_id) AS latest
                    INTO captured FROM %1$I.deadline_profile_revisions r JOIN %1$I.deadline_profiles p ON p.id=r.profile_id
                    WHERE r.profile_id=selected_id AND r.revision=(observation->>'revision')::bigint;
            WHEN 'resolution','notification' THEN
                SELECT r.revision,r.status,r.submission_digest,r.case_id,NULL::uuid AS hearing_id,
                    (r.values_view->'resolution'->>'id')::uuid AS parent_id,
                    (r.values_view->'resolution'->>'revision')::bigint AS parent_revision,
                    (SELECT max(revision) FROM %1$I.case_procedural_fact_revisions WHERE family=r.family AND id=r.id) AS latest
                    INTO captured FROM %1$I.case_procedural_fact_revisions r
                    WHERE r.family=observed_family AND r.id=selected_id AND r.revision=(observation->>'revision')::bigint;
            WHEN 'hearing_result' THEN
                SELECT r.revision,r.status,r.submission_digest,r.case_id,r.hearing_id,
                    NULL::uuid AS parent_id,NULL::bigint AS parent_revision,
                    (SELECT max(revision) FROM %1$I.case_hearing_result_revisions WHERE result_id=r.result_id) AS latest
                    INTO captured FROM %1$I.case_hearing_result_revisions r
                    WHERE r.result_id=selected_id AND r.revision=(observation->>'revision')::bigint;
            WHEN 'calendar' THEN
                SELECT r.revision,r.status,r.submission_digest,NULL::uuid AS case_id,NULL::uuid AS hearing_id,
                    NULL::uuid AS parent_id,NULL::bigint AS parent_revision,
                    (SELECT max(revision) FROM %1$I.judicial_calendar_revisions WHERE calendar_id=r.calendar_id) AS latest
                    INTO captured FROM %1$I.judicial_calendar_revisions r
                    WHERE r.calendar_id=selected_id AND r.revision=(observation->>'revision')::bigint;
            END CASE;
            IF captured.revision IS NULL OR captured.case_id IS DISTINCT FROM expected_case
                OR captured.hearing_id IS DISTINCT FROM expected_hearing
                OR captured.submission_digest IS DISTINCT FROM decode(observation->>'submission_digest','hex')
                OR captured.parent_id IS DISTINCT FROM (observation->'parent_resolution'->>'id')::uuid
                OR captured.parent_revision IS DISTINCT FROM (observation->'parent_resolution'->>'revision')::bigint THEN
                RAISE EXCEPTION 'observed dependency does not match its exact persisted receipt' USING ERRCODE='23514'; END IF;
            IF observed_role='source' AND observed_family='notification'
                AND captured.parent_id IS DISTINCT FROM NEW.source_parent_resolution_id THEN
                RAISE EXCEPTION 'observed notification changed its resolution root' USING ERRCODE='23514'; END IF;
            IF qualification AND (captured.revision IS DISTINCT FROM captured.latest
                OR captured.status IS DISTINCT FROM CASE WHEN observed_family IN ('profile','calendar') THEN 'published' ELSE 'recorded' END) THEN
                RAISE EXCEPTION 'human qualification requires current available observed dependencies' USING ERRCODE='23514'; END IF;
            IF (observed_role='source' AND (captured.revision<NEW.source_head_revision
                    OR (qualification AND captured.revision IS DISTINCT FROM NEW.source_head_revision)))
                OR (observed_role='calendar' AND (captured.revision<NEW.calendar_head_revision
                    OR (qualification AND captured.revision IS DISTINCT FROM NEW.calendar_head_revision))) THEN
                RAISE EXCEPTION 'tracking observation regresses or differs from the captured calculation head' USING ERRCODE='23514'; END IF;
            IF legacy_upgrade AND NOT qualification AND (
                (observed_role='profile' AND captured.revision IS DISTINCT FROM NEW.profile_revision)
                OR (observed_role='source' AND captured.revision IS DISTINCT FROM NEW.source_head_revision)
                OR (observed_role='calendar' AND captured.revision IS DISTINCT FROM NEW.calendar_head_revision)) THEN
                RAISE EXCEPTION 'legacy followup must observe its exact historical dependency captures' USING ERRCODE='23514'; END IF;
        END LOOP;
        IF legacy_upgrade AND NOT qualification AND parent_present THEN
            RAISE EXCEPTION 'legacy followup cannot add an unobserved notification parent' USING ERRCODE='23514'; END IF;
        IF source_present IS DISTINCT FROM (NEW.source_kind IS NOT NULL)
            OR calendar_present IS DISTINCT FROM (NEW.calendar_id IS NOT NULL)
            OR (qualification AND NEW.source_kind='notification' AND NOT parent_present) THEN
            RAISE EXCEPTION 'observed dependency presence differs from the captured input' USING ERRCODE='23514'; END IF;
    END IF;
    IF (NEW.action IN ('correct','retire') AND NEW.attention IS DISTINCT FROM previous.attention)
        OR (NEW.action='register' AND NEW.attention<>jsonb_build_object('status','pending')) THEN
        RAISE EXCEPTION 'deadline action cannot change attention' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS deadline_sequence ON case_deadline_revisions;
CREATE TRIGGER deadline_sequence BEFORE INSERT ON case_deadline_revisions
    FOR EACH ROW EXECUTE FUNCTION enforce_deadline_sequence();
REVOKE ALL ON FUNCTION enforce_deadline_sequence() FROM PUBLIC;
