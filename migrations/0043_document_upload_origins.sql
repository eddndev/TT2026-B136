CREATE TABLE IF NOT EXISTS document_upload_origins (
    document_id UUID CONSTRAINT document_upload_origin_primary PRIMARY KEY,
    case_id UUID NOT NULL,
    actor_id UUID NOT NULL,
    actor_email TEXT NOT NULL,
    recorded_at_seconds BIGINT NOT NULL,
    recorded_at_nanoseconds INTEGER NOT NULL,
    audit_sequence BIGINT NOT NULL CONSTRAINT document_upload_origin_audit_unique UNIQUE,
    CONSTRAINT document_upload_origin_scope FOREIGN KEY(document_id,case_id)
        REFERENCES document_series(id,case_id),
    CONSTRAINT document_upload_origin_actor FOREIGN KEY(actor_id) REFERENCES users(id),
    CONSTRAINT document_upload_origin_audit FOREIGN KEY(audit_sequence) REFERENCES audit_events(sequence),
    CONSTRAINT document_upload_origin_email CHECK(case_administration_text_valid(actor_email,320,FALSE)),
    CONSTRAINT document_upload_origin_seconds_min CHECK(recorded_at_seconds>=-62135596800),
    CONSTRAINT document_upload_origin_seconds_max CHECK(recorded_at_seconds<=253402300799),
    CONSTRAINT document_upload_origin_nanos_min CHECK(recorded_at_nanoseconds>=0),
    CONSTRAINT document_upload_origin_nanos_max CHECK(recorded_at_nanoseconds<=999999999)
);
REVOKE ALL ON document_upload_origins FROM PUBLIC;

CREATE OR REPLACE FUNCTION preserve_document_upload_origin_history() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'document upload origins are immutable' USING ERRCODE = '23514';
END;
$function$;

DO $migration$
BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_document_upload_origin() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
DECLARE actor RECORD; source RECORD; event RECORD; recorded TEXT;
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'document upload origins require read committed' USING ERRCODE = '23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.actor_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email COLLATE "C" IS DISTINCT FROM NEW.actor_email COLLATE "C"
        OR NOT(actor.role='owner' OR (actor.role IN ('litigator','paralegal') AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.actor_id FOR SHARE))) THEN
        RAISE EXCEPTION 'document uploader is not currently authorized' USING ERRCODE = '42501';
    END IF;
    SELECT d.digest INTO source FROM %1$I.document_series s JOIN %1$I.documents d
        ON d.id=s.id AND d.case_id=s.case_id AND d.version=1
        WHERE s.id=NEW.document_id AND s.case_id=NEW.case_id AND s.first_available_version=1;
    SELECT a.actor,a.action,a.resource,a.timestamp INTO event FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence;
    recorded:=to_char(to_timestamp(NEW.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
        ||CASE WHEN NEW.recorded_at_nanoseconds=0 THEN 'Z'
            ELSE '.'||rtrim(lpad(NEW.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END;
    IF source.digest IS NULL OR event.actor COLLATE "C" IS DISTINCT FROM NEW.actor_email COLLATE "C"
        OR event.action COLLATE "C" IS DISTINCT FROM 'document.uploaded' COLLATE "C"
        OR event.timestamp COLLATE "C" IS DISTINCT FROM recorded COLLATE "C"
        OR event.resource COLLATE "C" IS DISTINCT FROM ('case:'||NEW.case_id::text||':document:'
            ||NEW.document_id::text||':version:1:sha256:'||encode(source.digest,'hex')) COLLATE "C" THEN
        RAISE EXCEPTION 'document upload origin differs from its committed evidence' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$, current_schema());
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='document_upload_origins'::regclass
        AND tgname='document_upload_origin_immutable') THEN
        CREATE TRIGGER document_upload_origin_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
            ON document_upload_origins FOR EACH STATEMENT EXECUTE FUNCTION preserve_document_upload_origin_history();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='document_upload_origins'::regclass
        AND tgname='document_upload_origin_insert') THEN
        CREATE TRIGGER document_upload_origin_insert BEFORE INSERT ON document_upload_origins
            FOR EACH ROW EXECUTE FUNCTION enforce_document_upload_origin();
    END IF;
END;
$migration$;
REVOKE ALL ON FUNCTION preserve_document_upload_origin_history(),enforce_document_upload_origin() FROM PUBLIC;
