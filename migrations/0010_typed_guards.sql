-- Both revision families form one immutable participant history.
ALTER TABLE case_participants DROP CONSTRAINT IF EXISTS participant_first_revision_fk;
CREATE OR REPLACE FUNCTION preserve_typed_history() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'represented identity and typed history are immutable' USING ERRCODE='23514';
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_actor_guard(scope UUID,actor_id UUID,actor_email TEXT)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE actor RECORD;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'typed changes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=actor_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM actor_email
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=scope AND user_id=actor_id))) THEN
        RAISE EXCEPTION 'typed actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    IF COALESCE((SELECT administrative_status FROM %1$I.case_administration_revisions
        WHERE case_id=scope ORDER BY revision DESC LIMIT 1),'active')<>'active' THEN
        RAISE EXCEPTION 'case administration is closed' USING ERRCODE='23514';
    END IF;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_support_guard(scope UUID,support JSONB)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM %1$I.documents WHERE case_id=scope
        AND id=(support->>'document_id')::uuid AND version=(support->>'version')::bigint
        AND digest=decode(support->>'digest','hex')) THEN
        RAISE EXCEPTION 'typed support must match exact case content' USING ERRCODE='23514';
    END IF;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_subject_sequence() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE previous RECORD;scope UUID;value JSONB;
BEGIN
    SELECT case_id INTO scope FROM %1$I.case_subjects WHERE id=NEW.subject_id;
    PERFORM %1$I.typed_actor_guard(scope,NEW.changed_by,NEW.changed_by_email);
    SELECT revision,subject_kind INTO previous FROM %1$I.case_subject_revisions
        WHERE subject_id=NEW.subject_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM COALESCE(previous.revision+1,1)
        OR (previous.revision IS NOT NULL AND previous.subject_kind IS DISTINCT FROM NEW.subject_kind) THEN
        RAISE EXCEPTION 'subject revisions must be contiguous and keep their kind' USING ERRCODE='23514';
    END IF;
    value:=%1$I.typed_subject_values(NEW.values_canonical);
    PERFORM %1$I.typed_support_guard(scope,value->'identity_support');
    RETURN NEW;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_participant_sequence() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE latest_revision BIGINT;
BEGIN
    PERFORM pg_advisory_xact_lock(280603412820);
    IF EXISTS(SELECT 1 FROM %1$I.case_participant_typed_revisions WHERE participant_id=NEW.participant_id) THEN
        RAISE EXCEPTION 'typed participant cannot return to manual history' USING ERRCODE='23514';
    END IF;
    SELECT MAX(revision) INTO latest_revision FROM %1$I.case_participant_revisions WHERE participant_id=NEW.participant_id;
    IF NEW.revision IS DISTINCT FROM COALESCE(latest_revision+1,1) THEN
        RAISE EXCEPTION 'participant revisions must be contiguous' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_typed_sequence() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE previous RECORD;bound RECORD;scope UUID;head BIGINT;value JSONB;
BEGIN
    SELECT case_id INTO scope FROM %1$I.case_participants WHERE id=NEW.participant_id;
    PERFORM %1$I.typed_actor_guard(scope,NEW.changed_by,NEW.changed_by_email);
    SELECT MAX(revision) INTO head FROM (
        SELECT revision FROM %1$I.case_participant_revisions WHERE participant_id=NEW.participant_id
        UNION ALL SELECT revision FROM %1$I.case_participant_typed_revisions WHERE participant_id=NEW.participant_id) h;
    IF NEW.revision IS DISTINCT FROM COALESCE(head+1,1) THEN
        RAISE EXCEPTION 'typed participant revisions must be contiguous' USING ERRCODE='23514';
    END IF;
    SELECT r.*,s.case_id INTO bound FROM %1$I.case_subject_revisions r
        JOIN %1$I.case_subjects s ON s.id=r.subject_id WHERE r.subject_id=NEW.subject_id AND r.revision=NEW.subject_revision;
    IF bound.case_id IS DISTINCT FROM scope OR bound.values_digest IS DISTINCT FROM substring(NEW.values_canonical FROM 26 FOR 32)
        OR (NEW.role_kind='trial_court' AND bound.subject_kind<>'institutional_body')
        OR (NEW.role_kind NOT IN ('trial_court','other') AND bound.subject_kind<>'natural_person')
        OR (bound.subject_kind='institutional_body' AND NEW.credential_origin_revision IS NOT NULL) THEN
        RAISE EXCEPTION 'typed role must bind compatible exact case identity' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_participant_typed_revisions
        WHERE participant_id=NEW.participant_id ORDER BY revision DESC LIMIT 1;
    IF previous.revision IS NOT NULL AND previous.subject_id IS DISTINCT FROM NEW.subject_id THEN
        RAISE EXCEPTION 'typed identity root cannot be reassigned' USING ERRCODE='23514';
    END IF;
    IF NEW.submission_revision=NEW.revision THEN
        IF NEW.subject_revision<>(SELECT MAX(revision) FROM %1$I.case_subject_revisions WHERE subject_id=NEW.subject_id)
            OR (NEW.credential_origin_revision IS NOT NULL AND NEW.credential_origin_revision<>NEW.revision)
            OR (NEW.role_kind IN ('defense_counsel','control_judge') AND NEW.credential_origin_revision IS DISTINCT FROM NEW.revision)
            OR (head IS NOT NULL AND NEW.directory_status IS DISTINCT FROM COALESCE(previous.directory_status,
                (SELECT directory_status FROM %1$I.case_participant_revisions WHERE participant_id=NEW.participant_id ORDER BY revision DESC LIMIT 1))) THEN
            RAISE EXCEPTION 'typed replacement must use current identity and preserve directory status' USING ERRCODE='23514';
        END IF;
    ELSIF previous.revision IS NULL OR NEW.submission_revision IS DISTINCT FROM previous.submission_revision
        OR NEW.submission_digest IS DISTINCT FROM previous.submission_digest
        OR NEW.credential_origin_revision IS DISTINCT FROM previous.credential_origin_revision
        OR set_byte(NEW.values_canonical,57,get_byte(previous.values_canonical,57)) IS DISTINCT FROM previous.values_canonical THEN
        RAISE EXCEPTION 'status revision must preserve the accepted values and proof origin' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.case_participant_typed_revisions r WHERE r.subject_id=NEW.subject_id
        AND r.role_kind=NEW.role_kind AND r.participant_id<>NEW.participant_id
        AND r.revision=(SELECT MAX(h.revision) FROM %1$I.case_participant_typed_revisions h WHERE h.participant_id=r.participant_id)) THEN
        RAISE EXCEPTION 'represented identity already has this current role' USING ERRCODE='23505';
    END IF;
    value:=%1$I.typed_participant_values(NEW.values_canonical);
    PERFORM %1$I.typed_support_guard(scope,value->'role_support');
    IF value->'profile'->'contact'?'documented' THEN
        PERFORM %1$I.typed_support_guard(scope,value->'profile'->'contact'->'documented');
    END IF;
    IF value->'profile'->'protection'?'documented' THEN
        PERFORM %1$I.typed_support_guard(scope,value->'profile'->'protection'->'documented');
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_root_complete() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE count_first BIGINT;
BEGIN
    IF TG_TABLE_NAME='case_subjects' THEN
        IF NOT EXISTS(SELECT 1 FROM %1$I.case_participant_typed_revisions WHERE subject_id=NEW.id) THEN
            RAISE EXCEPTION 'new represented identity must be bound to a typed role' USING ERRCODE='23514';
        END IF;
    ELSE
        SELECT count(*) INTO count_first FROM (
            SELECT revision FROM %1$I.case_participant_revisions WHERE participant_id=NEW.id AND revision=1
            UNION ALL SELECT revision FROM %1$I.case_participant_typed_revisions WHERE participant_id=NEW.id AND revision=1) h;
        IF count_first<>1 THEN RAISE EXCEPTION 'participant requires exactly one initial revision' USING ERRCODE='23514'; END IF;
    END IF;
    RETURN NULL;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_subjects','case_subject_revisions','case_participant_typed_revisions',
        'subject_identity_reviews','participant_identity_reviews','participant_credential_evidence'] LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS typed_immutable ON %I',tab);
        EXECUTE format('CREATE TRIGGER typed_immutable BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION preserve_typed_history()',tab);
    END LOOP;
END; $$;
DROP TRIGGER IF EXISTS subject_sequence ON case_subject_revisions;
CREATE TRIGGER subject_sequence BEFORE INSERT ON case_subject_revisions FOR EACH ROW EXECUTE FUNCTION enforce_subject_sequence();
DROP TRIGGER IF EXISTS typed_sequence ON case_participant_typed_revisions;
CREATE TRIGGER typed_sequence BEFORE INSERT ON case_participant_typed_revisions FOR EACH ROW EXECUTE FUNCTION enforce_typed_sequence();
DROP TRIGGER IF EXISTS typed_root_complete ON case_participants;
CREATE CONSTRAINT TRIGGER typed_root_complete AFTER INSERT ON case_participants DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION typed_root_complete();
DROP TRIGGER IF EXISTS subject_root_complete ON case_subjects;
CREATE CONSTRAINT TRIGGER subject_root_complete AFTER INSERT ON case_subjects DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION typed_root_complete();
REVOKE ALL ON FUNCTION preserve_typed_history(),typed_actor_guard(UUID,UUID,TEXT),typed_support_guard(UUID,JSONB),
    enforce_subject_sequence(),enforce_typed_sequence(),typed_root_complete() FROM PUBLIC;
