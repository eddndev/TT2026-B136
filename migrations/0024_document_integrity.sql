CREATE TABLE IF NOT EXISTS document_integrity_incidents (
    id UUID CONSTRAINT document_integrity_primary PRIMARY KEY,
    observation_id UUID NOT NULL CONSTRAINT document_integrity_observation UNIQUE,
    case_id UUID NOT NULL,
    document_id UUID NOT NULL,
    document_version BIGINT NOT NULL,
    requester UUID NOT NULL,
    failure TEXT NOT NULL,
    detected_at_seconds BIGINT NOT NULL,
    detected_at_nanoseconds INTEGER NOT NULL,
    recorded_at_seconds BIGINT NOT NULL,
    recorded_at_nanoseconds INTEGER NOT NULL,
    expected_digest BYTEA NOT NULL,
    observed_snapshot_digest BYTEA NOT NULL,
    capture_canonical BYTEA NOT NULL,
    capture_digest BYTEA NOT NULL,
    CONSTRAINT document_integrity_scope FOREIGN KEY(document_id,case_id)
        REFERENCES document_series(id,case_id),
    CONSTRAINT document_integrity_version FOREIGN KEY(document_id,document_version)
        REFERENCES documents(id,version),
    CONSTRAINT document_integrity_requester FOREIGN KEY(requester) REFERENCES users(id),
    CONSTRAINT document_integrity_version_valid CHECK(document_version >= 1 AND document_version <= 4294967295),
    CONSTRAINT document_integrity_failure_valid CHECK(failure IN ('malformed_vault','authentication_failed','digest_mismatch','snapshot_changed')),
    CONSTRAINT document_integrity_detected_seconds CHECK(detected_at_seconds >= -62135596800 AND detected_at_seconds <= 253402300799),
    CONSTRAINT document_integrity_detected_nanos CHECK(detected_at_nanoseconds >= 0 AND detected_at_nanoseconds <= 999999999),
    CONSTRAINT document_integrity_recorded_seconds CHECK(recorded_at_seconds >= -62135596800 AND recorded_at_seconds <= 253402300799),
    CONSTRAINT document_integrity_recorded_nanos CHECK(recorded_at_nanoseconds >= 0 AND recorded_at_nanoseconds <= 999999999),
    CONSTRAINT document_integrity_chronology CHECK(ROW(recorded_at_seconds,recorded_at_nanoseconds) >= ROW(detected_at_seconds,detected_at_nanoseconds)),
    CONSTRAINT document_integrity_expected_digest CHECK(octet_length(expected_digest)=32),
    CONSTRAINT document_integrity_snapshot_digest CHECK(octet_length(observed_snapshot_digest)=32),
    CONSTRAINT document_integrity_capture_size CHECK(octet_length(capture_canonical)=178),
    CONSTRAINT document_integrity_capture_digest CHECK(capture_digest=pg_catalog.sha256(capture_canonical))
);
REVOKE ALL ON document_integrity_incidents FROM PUBLIC;

CREATE OR REPLACE FUNCTION preserve_document_integrity_history() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'document integrity incidents are immutable' USING ERRCODE = '23514';
END;
$function$;

CREATE OR REPLACE FUNCTION lock_document_integrity_history() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'document integrity writes require read committed' USING ERRCODE = '23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END;
$function$;

DO $migration$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid='document_integrity_incidents'::regclass AND tgname='document_integrity_immutable') THEN
        CREATE TRIGGER document_integrity_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
            ON document_integrity_incidents FOR EACH STATEMENT EXECUTE FUNCTION preserve_document_integrity_history();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid='document_integrity_incidents'::regclass AND tgname='document_integrity_lock') THEN
        CREATE TRIGGER document_integrity_lock BEFORE INSERT ON document_integrity_incidents
            FOR EACH STATEMENT EXECUTE FUNCTION lock_document_integrity_history();
    END IF;
END;
$migration$;
REVOKE ALL ON FUNCTION preserve_document_integrity_history(),lock_document_integrity_history() FROM PUBLIC;
