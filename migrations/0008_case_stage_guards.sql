CREATE OR REPLACE FUNCTION preserve_case_stage_history() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'case stage history is immutable' USING ERRCODE='23514';
END; $$;
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_case_stage_sequence() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE previous RECORD; administration RECORD; actor RECORD;
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'case stage writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    -- Match crates/infrastructure/src/audit_postgres.rs before reading mutable heads.
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    SELECT revision,stage INTO previous FROM (
        SELECT revision,stage FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id
        UNION ALL SELECT stage_revision,stage FROM %1$I.case_initial_stage_registrations WHERE case_id=NEW.case_id
    ) history ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM COALESCE(previous.revision+1,1) THEN
        RAISE EXCEPTION 'case stage revisions must be contiguous' USING ERRCODE='23514';
    END IF;
    IF previous.revision IS NULL THEN
        IF NEW.change_kind IS DISTINCT FROM 'adoption' OR NEW.from_stage IS NOT NULL THEN
            RAISE EXCEPTION 'unregistered stage requires explicit adoption' USING ERRCODE='23514';
        END IF;
    ELSIF NEW.from_stage IS DISTINCT FROM previous.stage
        OR NOT ((previous.stage='investigation' AND NEW.change_kind='to_intermediate')
            OR (previous.stage='intermediate' AND NEW.change_kind='to_trial')) THEN
        RAISE EXCEPTION 'stage transition does not follow the current stage' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.cases c JOIN %1$I.case_administration_revisions r ON r.case_id=c.id AND r.revision=1
        WHERE c.id=NEW.case_id AND c.required_initial_revision=1 AND r.nuc IS NOT NULL)
        AND NOT EXISTS(SELECT 1 FROM %1$I.case_initial_stage_registrations WHERE case_id=NEW.case_id) THEN
        RAISE EXCEPTION 'required initial registration cannot be replaced by adoption' USING ERRCODE='23514';
    END IF;
    SELECT revision,nuc,administrative_status INTO administration FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF administration.revision IS NULL OR administration.revision IS DISTINCT FROM NEW.administration_revision
        OR administration.nuc IS NULL OR administration.administrative_status IS DISTINCT FROM 'active' THEN
        RAISE EXCEPTION 'stage requires the current active complete administration' USING ERRCODE='23514';
    END IF;
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.recorded_by_email
        OR NOT (actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.recorded_by))) THEN
        RAISE EXCEPTION 'stage actor must match the currently authorized identity' USING ERRCODE='42501';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.documents WHERE id=NEW.support_id AND version=NEW.support_version
        AND case_id=NEW.case_id AND digest=NEW.support_digest AND name=NEW.support_name COLLATE "C")
        OR (NEW.receipt_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM %1$I.documents
            WHERE id=NEW.receipt_id AND version=NEW.receipt_version AND case_id=NEW.case_id
                AND digest=NEW.receipt_digest AND name=NEW.receipt_name COLLATE "C")) THEN
        RAISE EXCEPTION 'stage support must match exact case content' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$, current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.preserve_case_stage_initial_exclusivity() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'case stage writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    IF EXISTS(SELECT 1 FROM %1$I.case_stage_revisions WHERE case_id=NEW.case_id) THEN
        RAISE EXCEPTION 'initial registration cannot follow an adopted stage' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$, current_schema());
END; $install$;
DROP TRIGGER IF EXISTS case_stage_immutable ON case_stage_revisions;
CREATE TRIGGER case_stage_immutable BEFORE UPDATE OR DELETE ON case_stage_revisions
    FOR EACH ROW EXECUTE FUNCTION preserve_case_stage_history();
DROP TRIGGER IF EXISTS case_stage_sequence ON case_stage_revisions;
CREATE TRIGGER case_stage_sequence BEFORE INSERT ON case_stage_revisions
    FOR EACH ROW EXECUTE FUNCTION enforce_case_stage_sequence();
DROP TRIGGER IF EXISTS case_stage_initial_exclusive ON case_initial_stage_registrations;
CREATE TRIGGER case_stage_initial_exclusive BEFORE INSERT ON case_initial_stage_registrations
    FOR EACH ROW EXECUTE FUNCTION preserve_case_stage_initial_exclusivity();
REVOKE ALL ON FUNCTION preserve_case_stage_history(),enforce_case_stage_sequence(),
    preserve_case_stage_initial_exclusivity() FROM PUBLIC;
