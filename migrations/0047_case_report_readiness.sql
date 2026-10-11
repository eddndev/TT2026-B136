ALTER TABLE case_report_jobs ADD COLUMN IF NOT EXISTS estimated_total_ms BIGINT
    CHECK(estimated_total_ms > 0);

ALTER TABLE case_report_jobs ADD COLUMN IF NOT EXISTS estimated_service_ms BIGINT
    CHECK(estimated_service_ms > 0);

CREATE TABLE IF NOT EXISTS case_report_email_deliveries (
    report_id UUID PRIMARY KEY REFERENCES case_report_jobs(id),
    delivery_id UUID UNIQUE NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending','accepted','rejected','uncertain','access_revoked')),
    generation BIGINT NOT NULL DEFAULT 0 CHECK(generation >= 0),
    lease_id UUID,
    lease_until TEXT,
    provider_deadline TEXT,
    retry_at TEXT,
    message JSONB CHECK(jsonb_typeof(message)='object' AND octet_length(message::text)<=16384),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK((lease_id IS NULL)=(lease_until IS NULL)),
    CHECK((message IS NULL)=(provider_deadline IS NULL)),
    CHECK(lease_id IS NULL OR (state='pending' AND message IS NOT NULL)),
    CHECK(retry_at IS NULL OR (state='uncertain' AND lease_id IS NULL AND message IS NOT NULL))
);
CREATE OR REPLACE FUNCTION guard_case_report_email() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF NEW.report_id<>OLD.report_id OR NEW.delivery_id<>OLD.delivery_id
        OR NEW.created_at<>OLD.created_at
        OR (OLD.message IS NOT NULL AND NEW.message IS DISTINCT FROM OLD.message)
        OR (OLD.provider_deadline IS NOT NULL AND NEW.provider_deadline IS DISTINCT FROM OLD.provider_deadline)
        OR NEW.updated_at::timestamptz<OLD.updated_at::timestamptz
        OR NEW.generation<OLD.generation OR NEW.generation::numeric>OLD.generation::numeric+1
        OR (OLD.state IN ('accepted','rejected','access_revoked') AND NEW IS DISTINCT FROM OLD)
        OR (OLD.state='uncertain' AND OLD.retry_at IS NULL AND NEW IS DISTINCT FROM OLD) THEN
        RAISE EXCEPTION 'invalid ready report email transition' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
DO $migration$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_report_email_deliveries'::regclass AND tgname='case_report_write_lock') THEN
        CREATE TRIGGER case_report_write_lock BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON case_report_email_deliveries FOR EACH STATEMENT EXECUTE FUNCTION lock_case_report_writes();
        CREATE TRIGGER case_report_no_delete BEFORE DELETE OR TRUNCATE ON case_report_email_deliveries FOR EACH STATEMENT EXECUTE FUNCTION preserve_case_report_rows();
        CREATE TRIGGER case_report_email_guard BEFORE UPDATE ON case_report_email_deliveries FOR EACH ROW EXECUTE FUNCTION guard_case_report_email();
    END IF;
END;
$migration$;
REVOKE ALL ON case_report_email_deliveries FROM PUBLIC;
REVOKE ALL ON FUNCTION guard_case_report_email() FROM PUBLIC;
