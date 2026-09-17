-- Appended hearing history cannot be rewritten, deleted or truncated.
CREATE OR REPLACE FUNCTION preserve_hearing_history() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'hearing history is immutable' USING ERRCODE='23514';
END; $$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['case_hearings','case_hearing_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='hearing_immutable') THEN
            EXECUTE format('CREATE TRIGGER hearing_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_hearing_history()',tab);
        END IF;
    END LOOP;
END; $$;
REVOKE ALL ON FUNCTION preserve_hearing_history() FROM PUBLIC;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_hearing_sequence() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE previous RECORD; administration RECORD; actor RECORD; current_stage RECORD;
    value JSONB; selected JSONB; participant RECORD; source_count BIGINT;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'hearing writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.recorded_by_email
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by))) THEN
        RAISE EXCEPTION 'hearing actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS DISTINCT FROM NEW.recorded_administration_revision
        OR administration.values_digest IS DISTINCT FROM NEW.recorded_administration_digest
        OR administration.administrative_status IS DISTINCT FROM 'active' THEN
        RAISE EXCEPTION 'hearing recording requires current active administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO previous FROM %1$I.case_hearing_revisions
        WHERE hearing_id=NEW.hearing_id ORDER BY revision DESC LIMIT 1;
    value:=%1$I.hearing_values(NEW.values_canonical);
    IF NEW.revision IS DISTINCT FROM COALESCE(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'schedule')
        OR (previous.revision IS NOT NULL AND (NEW.action='schedule' OR previous.status<>'scheduled'
            OR previous.case_id IS DISTINCT FROM NEW.case_id OR previous.values_view->>'kind' IS DISTINCT FROM value->>'kind')) THEN
        RAISE EXCEPTION 'hearing revisions must follow a scheduled predecessor of the same kind' USING ERRCODE='23514';
    END IF;
    IF NEW.action='cancel' THEN
        IF ROW(NEW.values_canonical,NEW.values_digest,NEW.scheduling_administration_revision,
            NEW.scheduling_administration_digest,NEW.scheduling_stage_revision,NEW.scheduling_stage,
            NEW.scheduling_stage_digest,NEW.support_name,NEW.support_format,NEW.support_policy)
            IS DISTINCT FROM ROW(previous.values_canonical,previous.values_digest,previous.scheduling_administration_revision,
                previous.scheduling_administration_digest,previous.scheduling_stage_revision,previous.scheduling_stage,
                previous.scheduling_stage_digest,previous.support_name,previous.support_format,previous.support_policy) THEN
            RAISE EXCEPTION 'cancellation must preserve the exact prior programming' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.scheduling_administration_revision IS DISTINCT FROM administration.revision
        OR NEW.scheduling_administration_digest IS DISTINCT FROM administration.values_digest
        OR administration.nuc IS NULL THEN
        RAISE EXCEPTION 'hearing scheduling requires current complete administration' USING ERRCODE='23514';
    END IF;
    SELECT * INTO current_stage FROM (
        SELECT revision,stage,values_digest FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id
        UNION ALL SELECT stage_revision,stage,NULL::bytea FROM %1$I.case_initial_stage_registrations WHERE case_id=NEW.case_id
    ) stages ORDER BY revision DESC LIMIT 1;
    IF current_stage.revision IS DISTINCT FROM NEW.scheduling_stage_revision
        OR current_stage.stage IS DISTINCT FROM NEW.scheduling_stage
        OR current_stage.values_digest IS DISTINCT FROM NEW.scheduling_stage_digest THEN
        RAISE EXCEPTION 'hearing scheduling requires the exact current stage' USING ERRCODE='23514';
    END IF;
    FOR selected IN SELECT jsonb_array_elements(value->'participants') LOOP
        SELECT count(*) INTO source_count FROM (
            SELECT r.revision FROM %1$I.case_participant_revisions r
            JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=NEW.case_id AND p.id=(selected->>'id')::uuid AND r.revision=(selected->>'revision')::bigint
            UNION ALL SELECT r.revision FROM %1$I.case_participant_typed_revisions r
            JOIN %1$I.case_participants p ON p.id=r.participant_id
            WHERE p.case_id=NEW.case_id AND p.id=(selected->>'id')::uuid AND r.revision=(selected->>'revision')::bigint
        ) sources;
        IF source_count<>1 THEN
            RAISE EXCEPTION 'hearing participant source must belong to its case' USING ERRCODE='23514';
        END IF;
        IF previous.revision IS NULL OR NOT(previous.values_view->'participants' @> jsonb_build_array(selected)) THEN
            SELECT * INTO participant FROM (
                SELECT revision,directory_status FROM %1$I.case_participant_revisions WHERE participant_id=(selected->>'id')::uuid
                UNION ALL SELECT revision,directory_status FROM %1$I.case_participant_typed_revisions WHERE participant_id=(selected->>'id')::uuid
            ) revisions ORDER BY revision DESC LIMIT 1;
            IF participant.revision IS DISTINCT FROM (selected->>'revision')::bigint OR participant.directory_status IS DISTINCT FROM 'active' THEN
                RAISE EXCEPTION 'newly selected hearing participant must be current and active' USING ERRCODE='23514';
            END IF;
        END IF;
    END LOOP;
    IF value->>'kind'='sentencing' AND NOT EXISTS(SELECT 1 FROM %1$I.documents
        WHERE case_id=NEW.case_id AND id=(value->'conviction_basis'->>'document_id')::uuid
            AND version=(value->'conviction_basis'->>'version')::bigint
            AND digest=decode(value->'conviction_basis'->>'digest','hex') AND name=NEW.support_name COLLATE "C") THEN
        RAISE EXCEPTION 'hearing support must match exact case content' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS hearing_sequence ON case_hearing_revisions;
CREATE TRIGGER hearing_sequence BEFORE INSERT ON case_hearing_revisions FOR EACH ROW EXECUTE FUNCTION enforce_hearing_sequence();
REVOKE ALL ON FUNCTION enforce_hearing_sequence() FROM PUBLIC;
