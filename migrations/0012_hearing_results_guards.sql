-- Immutable sources plus a pre-existing predecessor prohibit cyclic continuation roots.
CREATE OR REPLACE FUNCTION preserve_hearing_result_history() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'hearing result history is immutable' USING ERRCODE='23514';
END; $$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_hearing_results','case_hearing_result_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='hearing_result_immutable') THEN
            EXECUTE format('CREATE TRIGGER hearing_result_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_hearing_result_history()',tab);
        END IF;
    END LOOP;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_hearing_result_root() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE anchor RECORD; prior RECORD;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'hearing result writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT * INTO anchor FROM %1$I.case_hearing_revisions
        WHERE case_id=NEW.case_id AND hearing_id=NEW.hearing_id AND revision=NEW.anchor_revision;
    IF anchor.values_digest IS DISTINCT FROM NEW.anchor_values_digest
        OR anchor.submission_digest IS DISTINCT FROM NEW.anchor_submission_digest THEN
        RAISE EXCEPTION 'result anchor must match existing exact programming' USING ERRCODE='23514';
    END IF;
    IF NEW.continuation_result_id IS NOT NULL THEN
        SELECT * INTO prior FROM %1$I.case_hearing_result_revisions
            WHERE case_id=NEW.case_id AND result_id=NEW.continuation_result_id AND revision=NEW.continuation_revision;
        IF NEW.continuation_result_id=NEW.id OR prior.hearing_id IS DISTINCT FROM NEW.continuation_hearing_id
            OR prior.values_digest IS DISTINCT FROM NEW.continuation_values_digest
            OR prior.submission_digest IS DISTINCT FROM NEW.continuation_submission_digest THEN
            RAISE EXCEPTION 'result continuation must already exist in the same case' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_hearing_result_sequence() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE previous RECORD; administration RECORD; actor RECORD; root RECORD;
    value JSONB; receipt JSONB; selected JSONB; source_count BIGINT; expected_continuation JSONB;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'hearing result writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.recorded_by_email
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by))) THEN
        RAISE EXCEPTION 'hearing result actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS DISTINCT FROM NEW.recorded_administration_revision
        OR administration.values_digest IS DISTINCT FROM NEW.recorded_administration_digest
        OR administration.administrative_status IS DISTINCT FROM 'active' THEN
        RAISE EXCEPTION 'hearing result recording requires current active administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO root FROM %1$I.case_hearing_results WHERE id=NEW.result_id;
    receipt:=%1$I.hearing_result_submission(NEW.submission_canonical);
    expected_continuation:=CASE WHEN root.continuation_result_id IS NULL THEN 'null'::jsonb ELSE
        jsonb_build_object('hearing_id',root.continuation_hearing_id,'result_id',root.continuation_result_id,
            'revision',root.continuation_revision,'values_digest',encode(root.continuation_values_digest,'hex'),
            'submission_digest',encode(root.continuation_submission_digest,'hex')) END;
    IF root.case_id IS DISTINCT FROM NEW.case_id OR root.hearing_id IS DISTINCT FROM NEW.hearing_id
        OR receipt->'anchor' IS DISTINCT FROM jsonb_build_object('revision',root.anchor_revision,
            'values_digest',encode(root.anchor_values_digest,'hex'),'submission_digest',encode(root.anchor_submission_digest,'hex'))
        OR receipt->'continuation' IS DISTINCT FROM expected_continuation THEN
        RAISE EXCEPTION 'result receipt must preserve the exact immutable sources' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_hearing_result_revisions
        WHERE result_id=NEW.result_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM COALESCE(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'record')
        OR (previous.revision IS NOT NULL AND (NEW.action='record' OR previous.status<>'recorded'
            OR previous.case_id IS DISTINCT FROM NEW.case_id OR previous.hearing_id IS DISTINCT FROM NEW.hearing_id)) THEN
        RAISE EXCEPTION 'result revisions must follow a recorded predecessor' USING ERRCODE='23514';
    END IF;
    IF NEW.action='withdraw' AND ROW(NEW.values_canonical,NEW.values_digest,NEW.support_name,NEW.support_format,NEW.support_policy)
        IS DISTINCT FROM ROW(previous.values_canonical,previous.values_digest,previous.support_name,previous.support_format,previous.support_policy) THEN
        RAISE EXCEPTION 'withdrawal must preserve exact prior declared content and support' USING ERRCODE='23514';
    END IF;
    value:=%1$I.hearing_result_values(NEW.values_canonical);
    FOR selected IN SELECT jsonb_array_elements(value->'attendees') LOOP
        SELECT count(*) INTO source_count FROM (
            SELECT r.revision FROM %1$I.case_participant_revisions r
            JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=NEW.case_id AND p.id=(selected->>'participant_id')::uuid AND r.revision=(selected->>'revision')::bigint
            UNION ALL SELECT r.revision FROM %1$I.case_participant_typed_revisions r
            JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=NEW.case_id AND p.id=(selected->>'participant_id')::uuid AND r.revision=(selected->>'revision')::bigint
        ) sources;
        IF source_count<>1 THEN
            RAISE EXCEPTION 'result attendee must refer to exact case history' USING ERRCODE='23514';
        END IF;
    END LOOP;
    selected:=value->'provenance'->'support';
    IF selected<>'null'::jsonb AND NOT EXISTS(SELECT 1 FROM %1$I.documents
        WHERE case_id=NEW.case_id AND id=(selected->>'document_id')::uuid
            AND version=(selected->>'version')::bigint AND digest=decode(selected->>'digest','hex')
            AND name=NEW.support_name COLLATE "C") THEN
        RAISE EXCEPTION 'result support must match exact case content' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS hearing_result_root_sources ON case_hearing_results;
CREATE TRIGGER hearing_result_root_sources BEFORE INSERT ON case_hearing_results FOR EACH ROW EXECUTE FUNCTION enforce_hearing_result_root();
DROP TRIGGER IF EXISTS hearing_result_sequence ON case_hearing_result_revisions;
CREATE TRIGGER hearing_result_sequence BEFORE INSERT ON case_hearing_result_revisions FOR EACH ROW EXECUTE FUNCTION enforce_hearing_result_sequence();
REVOKE ALL ON FUNCTION preserve_hearing_result_history(),enforce_hearing_result_root(),enforce_hearing_result_sequence() FROM PUBLIC;
