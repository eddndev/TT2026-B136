-- Mutations share the audit lock. Attention and retirement preserve the captured calculation.
CREATE OR REPLACE FUNCTION preserve_deadline_history()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'deadline history is immutable' USING ERRCODE='23514';
END; $$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_deadlines','case_deadline_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='deadline_immutable') THEN
            EXECUTE format('CREATE TRIGGER deadline_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_deadline_history()',tab);
        END IF;
    END LOOP;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_deadline_sequence()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; actor RECORD; responsible RECORD; baseline RECORD; current_admin RECORD;
    captured RECORD; profile RECORD; selected_source JSONB; head_source JSONB; selection JSONB;
    admin_bytes BYTEA; latest BIGINT; parent UUID;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'deadline writes require read committed isolation' USING ERRCODE='23514'; END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
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
    IF NEW.action IN ('register','correct') THEN
        IF NEW.observed_administration_revision IS DISTINCT FROM current_admin.revision
            OR NEW.profile_revision IS DISTINCT FROM profile.revision OR profile.status IS DISTINCT FROM 'published' THEN
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
REVOKE ALL ON FUNCTION preserve_deadline_history(),enforce_deadline_sequence() FROM PUBLIC;
