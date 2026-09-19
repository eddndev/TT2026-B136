CREATE OR REPLACE FUNCTION lock_alert_mutations() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM pg_advisory_xact_lock(280603412820); RETURN NULL; END
$$;
CREATE OR REPLACE FUNCTION preserve_alert_rows() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'alert evidence cannot be removed or replaced' USING ERRCODE='42501'; END
$$;
CREATE OR REPLACE FUNCTION validate_alert_update() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE mutable TEXT[]; old_json JSONB; new_json JSONB;
BEGIN
    old_json:=to_jsonb(OLD); new_json:=to_jsonb(NEW);
    mutable := CASE TG_TABLE_NAME
        WHEN 'alert_subject_state' THEN ARRAY['generation','dirty','payload','payload_digest']
        WHEN 'alert_scan_cursor' THEN ARRAY['kind','id','active_kind','active_id','after_recipient','cycle','next_seconds','next_nanos']
        WHEN 'alert_schedule' THEN ARRAY['generation','status','payload','payload_digest']
        WHEN 'alert_notifications' THEN ARRAY['read_seconds','read_nanos','resolved_seconds','resolved_nanos','resolved_reason']
        WHEN 'alert_email_outbox' THEN ARRAY['status','next_seconds','next_nanos','sequence','payload','payload_digest']
    END;
    IF mutable IS NULL OR (to_jsonb(NEW)-mutable) IS DISTINCT FROM (to_jsonb(OLD)-mutable) THEN
        RAISE EXCEPTION 'alert immutable projection changed' USING ERRCODE='23514'; END IF;
    IF TG_TABLE_NAME='alert_notifications' AND
        ((old_json->>'read_seconds' IS NOT NULL AND
            (new_json->'read_seconds',new_json->'read_nanos') IS DISTINCT FROM
            (old_json->'read_seconds',old_json->'read_nanos')) OR
         (old_json->>'resolved_seconds' IS NOT NULL AND
            (new_json->'resolved_seconds',new_json->'resolved_nanos',new_json->'resolved_reason') IS DISTINCT FROM
            (old_json->'resolved_seconds',old_json->'resolved_nanos',old_json->'resolved_reason'))) THEN
        RAISE EXCEPTION 'alert acknowledgement is monotonic' USING ERRCODE='23514'; END IF;
    IF TG_TABLE_NAME='alert_schedule' AND old_json->>'status'='activated' AND NEW IS DISTINCT FROM OLD THEN
        RAISE EXCEPTION 'completed alert plan cannot be rewritten' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END
$$;
CREATE OR REPLACE FUNCTION validate_alert_attempt() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous BIGINT;
BEGIN
    SELECT max(sequence) INTO previous FROM alert_email_attempts WHERE delivery_id=NEW.delivery_id;
    IF NEW.sequence<>coalesce(previous,-1)+1 THEN
        RAISE EXCEPTION 'alert delivery sequence is not consecutive' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END
$$;
CREATE OR REPLACE FUNCTION validate_alert_outbox_ledger() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE current_row alert_email_outbox;
BEGIN
    SELECT * INTO current_row FROM alert_email_outbox WHERE id=NEW.id;
    IF NOT EXISTS(SELECT 1 FROM alert_email_attempts a WHERE a.delivery_id=current_row.id
        AND a.sequence=current_row.sequence AND a.payload=current_row.payload
        AND a.payload_digest=current_row.payload_digest) THEN
        RAISE EXCEPTION 'alert outbox differs from delivery ledger' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END
$$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['alert_preferences','alert_subject_state','alert_scan_cursor',
        'alert_schedule','alert_notifications','alert_read_receipts','alert_email_outbox','alert_email_attempts'] LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS alert_lock ON %I',tab);
        EXECUTE format('CREATE TRIGGER alert_lock BEFORE INSERT OR UPDATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION lock_alert_mutations()',tab);
        EXECUTE format('DROP TRIGGER IF EXISTS alert_protected ON %I',tab);
        EXECUTE format('CREATE TRIGGER alert_protected BEFORE DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_alert_rows()',tab);
        EXECUTE format('DROP TRIGGER IF EXISTS alert_update ON %I',tab);
        EXECUTE format('CREATE TRIGGER alert_update BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION validate_alert_update()',tab);
    END LOOP;
END $$;
DROP TRIGGER IF EXISTS alert_attempt_insert ON alert_email_attempts;
CREATE TRIGGER alert_attempt_insert BEFORE INSERT ON alert_email_attempts
    FOR EACH ROW EXECUTE FUNCTION validate_alert_attempt();
DROP TRIGGER IF EXISTS alert_outbox_ledger ON alert_email_outbox;
CREATE CONSTRAINT TRIGGER alert_outbox_ledger AFTER INSERT OR UPDATE ON alert_email_outbox
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION validate_alert_outbox_ledger();
REVOKE ALL ON FUNCTION lock_alert_mutations(),preserve_alert_rows(),validate_alert_update(),
    validate_alert_attempt(),validate_alert_outbox_ledger() FROM PUBLIC;
