CREATE TABLE IF NOT EXISTS case_report_jobs (
    id UUID PRIMARY KEY,
    requester_id UUID NOT NULL REFERENCES users(id),
    principal JSONB NOT NULL CHECK(jsonb_typeof(principal)='object' AND octet_length(principal::text)<=2048),
    account_revision BIGINT NOT NULL CHECK(account_revision>=0),
    auth_generation BIGINT NOT NULL CHECK(auth_generation>=0),
    scope TEXT NOT NULL CHECK(scope IN ('office','assigned_cases')),
    operation_id UUID NOT NULL,
    command JSONB NOT NULL CHECK(jsonb_typeof(command)='object' AND octet_length(command::text)<=4096),
    request_digest BYTEA NOT NULL CHECK(octet_length(request_digest)=32),
    requested_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('queued','capturing','rendering','retry_capturing','retry_rendering','ready','failed','revoked')),
    failure TEXT CHECK(failure IN ('temporary_unavailable','render_unavailable','render_failed','capacity_exceeded','invalid_stored_capture','access_revoked')),
    retry_at TEXT,
    lease_attempt UUID,
    lease_token UUID,
    lease_generation BIGINT NOT NULL DEFAULT 0 CHECK(lease_generation>=0),
    lease_expires_at TEXT,
    attempts INTEGER NOT NULL DEFAULT 0 CHECK(attempts BETWEEN 0 AND 5),
    case_ids UUID[] NOT NULL DEFAULT '{}' CHECK(cardinality(case_ids)<=1000 AND array_position(case_ids,NULL) IS NULL),
    UNIQUE(requester_id,operation_id),
    CHECK((lease_attempt IS NULL)=(lease_token IS NULL) AND (lease_token IS NULL)=(lease_expires_at IS NULL)),
    CHECK((state IN ('capturing','rendering'))=(lease_attempt IS NOT NULL AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)),
    CHECK((state IN ('retry_capturing','retry_rendering'))=(retry_at IS NOT NULL)),
    CHECK((state IN ('failed','revoked','retry_capturing','retry_rendering'))=(failure IS NOT NULL))
);
CREATE TABLE IF NOT EXISTS case_report_snapshots (
    report_id UUID PRIMARY KEY REFERENCES case_report_jobs(id),
    digest BYTEA NOT NULL CHECK(octet_length(digest)=32),
    plaintext_digest BYTEA NOT NULL CHECK(octet_length(plaintext_digest)=32),
    checked_at TEXT NOT NULL,
    wrapped_dek BYTEA NOT NULL CHECK(octet_length(wrapped_dek)=60),
    payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 28 AND 8388636)
);
CREATE TABLE IF NOT EXISTS case_report_artifacts (
    report_id UUID NOT NULL REFERENCES case_report_snapshots(report_id),
    format TEXT NOT NULL CHECK(format IN ('pdf','csv')),
    snapshot_digest BYTEA NOT NULL CHECK(octet_length(snapshot_digest)=32),
    digest BYTEA NOT NULL CHECK(octet_length(digest)=32),
    bytes BIGINT NOT NULL CHECK(bytes BETWEEN 1 AND 16777216),
    wrapped_dek BYTEA NOT NULL CHECK(octet_length(wrapped_dek)=60),
    payload BYTEA NOT NULL CHECK(octet_length(payload)=bytes+28),
    PRIMARY KEY(report_id,format)
);
CREATE TABLE IF NOT EXISTS case_report_notices (
    report_id UUID PRIMARY KEY REFERENCES case_report_jobs(id),
    kind TEXT NOT NULL CHECK(kind IN ('ready','failed')),
    created_at TEXT NOT NULL,
    read_at TEXT
);
CREATE INDEX IF NOT EXISTS case_report_requester_page ON case_report_jobs(requester_id,id);
CREATE INDEX IF NOT EXISTS case_report_queue ON case_report_jobs(state,id);
CREATE INDEX IF NOT EXISTS case_report_unread ON case_report_notices(report_id) WHERE read_at IS NULL;

CREATE OR REPLACE FUNCTION lock_case_report_writes() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'report writes require read committed' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END;
$function$;
CREATE OR REPLACE FUNCTION preserve_case_report_rows() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'report evidence cannot be replaced or removed' USING ERRCODE='23514';
END;
$function$;
CREATE OR REPLACE FUNCTION guard_case_report_job() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF (to_jsonb(NEW)-ARRAY['updated_at','state','failure','retry_at','lease_attempt','lease_token','lease_generation','lease_expires_at','attempts','case_ids'])
        IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['updated_at','state','failure','retry_at','lease_attempt','lease_token','lease_generation','lease_expires_at','attempts','case_ids']) THEN
        RAISE EXCEPTION 'report request is immutable' USING ERRCODE='23514';
    END IF;
    IF NEW.updated_at::timestamptz<OLD.updated_at::timestamptz
        OR NEW.lease_generation<OLD.lease_generation OR NEW.attempts<OLD.attempts
        OR NEW.lease_generation::numeric>OLD.lease_generation::numeric+1
        OR NEW.attempts>OLD.attempts+1
        OR (NEW.case_ids IS DISTINCT FROM OLD.case_ids AND NOT(OLD.state='capturing' AND NEW.state='rendering'))
        OR OLD.state IN ('ready','failed','revoked') AND (to_jsonb(NEW)-'updated_at') IS DISTINCT FROM (to_jsonb(OLD)-'updated_at') THEN
        RAISE EXCEPTION 'invalid report transition' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
CREATE OR REPLACE FUNCTION guard_case_report_notice() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF NEW.report_id<>OLD.report_id OR NEW.kind<>OLD.kind OR NEW.created_at<>OLD.created_at
        OR OLD.read_at IS NOT NULL AND NEW.read_at IS DISTINCT FROM OLD.read_at
        OR NEW.read_at IS NULL OR NEW.read_at::timestamptz<NEW.created_at::timestamptz THEN
        RAISE EXCEPTION 'report notice acknowledgement is immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
DO $migration$
DECLARE table_name TEXT;
BEGIN
    FOREACH table_name IN ARRAY ARRAY['case_report_jobs','case_report_snapshots','case_report_artifacts','case_report_notices'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=table_name::regclass AND tgname='case_report_write_lock') THEN
            EXECUTE format('CREATE TRIGGER case_report_write_lock BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION lock_case_report_writes()',table_name);
            EXECUTE format('CREATE TRIGGER case_report_no_delete BEFORE DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_case_report_rows()',table_name);
        END IF;
    END LOOP;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_report_jobs'::regclass AND tgname='case_report_job_guard') THEN
        CREATE TRIGGER case_report_job_guard BEFORE UPDATE ON case_report_jobs FOR EACH ROW EXECUTE FUNCTION guard_case_report_job();
        CREATE TRIGGER case_report_notice_guard BEFORE UPDATE ON case_report_notices FOR EACH ROW EXECUTE FUNCTION guard_case_report_notice();
        CREATE TRIGGER case_report_snapshot_immutable BEFORE UPDATE ON case_report_snapshots FOR EACH ROW EXECUTE FUNCTION preserve_case_report_rows();
        CREATE TRIGGER case_report_artifact_immutable BEFORE UPDATE ON case_report_artifacts FOR EACH ROW EXECUTE FUNCTION preserve_case_report_rows();
    END IF;
END;
$migration$;
REVOKE ALL ON case_report_jobs,case_report_snapshots,case_report_artifacts,case_report_notices FROM PUBLIC;
REVOKE ALL ON FUNCTION lock_case_report_writes(),preserve_case_report_rows(),guard_case_report_job(),guard_case_report_notice() FROM PUBLIC;
