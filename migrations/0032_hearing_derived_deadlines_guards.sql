CREATE OR REPLACE FUNCTION preserve_hearing_derived_deadline_history()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'hearing derived deadline origins are immutable' USING ERRCODE='23514';
END; $$;
CREATE OR REPLACE FUNCTION lock_hearing_derived_deadline_history()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'hearing derived deadline writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_hearing_derived_deadline_origin()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE actor RECORD; result RECORD; deadline RECORD; event RECORD; audit RECORD;
    review_prefix BYTEA; capture BYTEA; instant BYTEA; remaining NUMERIC; byte_index INTEGER;
    recorded TEXT; marker TEXT;
BEGIN
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.actor_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE
        OR actor.email COLLATE "C" IS DISTINCT FROM NEW.actor_email COLLATE "C"
        OR actor.role COLLATE "C" IS DISTINCT FROM NEW.actor_role COLLATE "C"
        OR NOT(actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=NEW.case_id AND user_id=NEW.actor_id FOR SHARE))) THEN
        RAISE EXCEPTION 'hearing derived deadline actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO result FROM %1$I.case_hearing_result_revisions
        WHERE result_id=NEW.result_id AND revision=NEW.result_revision;
    SELECT * INTO deadline FROM %1$I.case_deadline_revisions
        WHERE deadline_id=NEW.deadline_id AND revision=NEW.deadline_revision;
    IF NEW.result_revision IS DISTINCT FROM 1 OR NEW.deadline_revision IS DISTINCT FROM 1
        OR result.operation_id IS DISTINCT FROM NEW.operation_id
        OR result.case_id IS DISTINCT FROM NEW.case_id OR result.hearing_id IS DISTINCT FROM NEW.hearing_id
        OR result.action IS DISTINCT FROM 'record' OR result.status IS DISTINCT FROM 'recorded'
        OR result.recorded_by IS DISTINCT FROM NEW.actor_id
        OR result.recorded_by_email COLLATE "C" IS DISTINCT FROM NEW.actor_email COLLATE "C"
        OR deadline.operation_id IS DISTINCT FROM NEW.deadline_operation_id
        OR deadline.case_id IS DISTINCT FROM NEW.case_id
        OR deadline.action IS DISTINCT FROM 'register' OR deadline.status IS DISTINCT FROM 'active'
        OR deadline.recorded_by IS DISTINCT FROM NEW.actor_id
        OR deadline.recorded_by_email COLLATE "C" IS DISTINCT FROM NEW.actor_email COLLATE "C"
        OR deadline.recorded_at_seconds IS DISTINCT FROM result.recorded_at_seconds
        OR deadline.recorded_at_nanoseconds IS DISTINCT FROM result.recorded_at_nanoseconds
        OR deadline.observed_administration_revision IS DISTINCT FROM result.recorded_administration_revision
        OR deadline.observed_administration_digest IS DISTINCT FROM result.recorded_administration_digest
        OR deadline.source_kind IS DISTINCT FROM 'hearing_result'
        OR deadline.source_id IS DISTINCT FROM NEW.result_id
        OR deadline.source_revision IS DISTINCT FROM NEW.result_revision
        OR deadline.source_head_revision IS DISTINCT FROM NEW.result_revision
        OR deadline.source_hearing_id IS DISTINCT FROM NEW.hearing_id
        OR substring(deadline.submission_canonical FROM 1 FOR 5) IS DISTINCT FROM convert_to('DLTX2','UTF8')
        OR deadline.submission_view->'predecessor' IS DISTINCT FROM 'null'::jsonb
        OR deadline.submission_view->'cause' IS DISTINCT FROM 'null'::jsonb THEN
        RAISE EXCEPTION 'hearing derived deadline components differ from their initial captures' USING ERRCODE='23514';
    END IF;
    SELECT * INTO event FROM %1$I.deadline_source_events WHERE sequence=NEW.source_event_sequence;
    IF event.source_kind IS DISTINCT FROM 'hearing_result' OR event.source_id IS DISTINCT FROM NEW.result_id
        OR event.revision IS DISTINCT FROM NEW.result_revision OR event.operation_id IS DISTINCT FROM NEW.operation_id
        OR event.case_id IS DISTINCT FROM NEW.case_id OR event.hearing_id IS DISTINCT FROM NEW.hearing_id THEN
        RAISE EXCEPTION 'hearing derived deadline requires its exact emitted result event' USING ERRCODE='23514';
    END IF;
    review_prefix:=convert_to('HRDL1','UTF8')||uuid_send(NEW.actor_id)
        ||int8send(octet_length(convert_to(NEW.actor_email,'UTF8'))::bigint)||convert_to(NEW.actor_email,'UTF8')
        ||int8send(octet_length(convert_to(NEW.actor_role,'UTF8'))::bigint)||convert_to(NEW.actor_role,'UTF8')
        ||uuid_send(NEW.case_id)||int4send(NEW.result_revision::integer)
        ||result.values_digest||result.submission_digest||result.recorded_administration_digest;
    IF substring(NEW.review_canonical FROM 1 FOR octet_length(review_prefix)) IS DISTINCT FROM review_prefix
        OR substring(NEW.review_canonical FROM octet_length(NEW.review_canonical)-31)
            IS DISTINCT FROM pg_catalog.sha256(deadline.result_canonical) THEN
        RAISE EXCEPTION 'hearing derived deadline review projections differ' USING ERRCODE='23514';
    END IF;
    -- Canonical instants use signed i128 nanoseconds and a zero UTC offset.
    remaining:=deadline.recorded_at_seconds::numeric*1000000000+deadline.recorded_at_nanoseconds;
    IF remaining<0 THEN remaining:=remaining+340282366920938463463374607431768211456; END IF;
    instant:=decode(repeat('00',16),'hex');
    FOR byte_index IN REVERSE 15..0 LOOP
        instant:=set_byte(instant,byte_index,mod(remaining,256)::integer);
        remaining:=trunc(remaining/256);
    END LOOP;
    capture:=convert_to('HRDC1','UTF8')||NEW.review_digest
        ||uuid_send(NEW.case_id)||uuid_send(NEW.hearing_id)||uuid_send(NEW.result_id)
        ||int4send(NEW.result_revision::integer)||uuid_send(NEW.operation_id)
        ||result.values_digest||result.submission_digest||int8send(NEW.source_event_sequence)||decode('02','hex')
        ||uuid_send(event.source_id)||int4send(event.revision::integer)||uuid_send(event.operation_id)
        ||int8send(octet_length(deadline.submission_canonical)::bigint)||deadline.submission_canonical
        ||deadline.submission_digest||deadline.capture_digest||instant||int4send(0);
    IF NEW.capture_canonical IS DISTINCT FROM capture THEN
        RAISE EXCEPTION 'hearing derived deadline capture differs from component receipts' USING ERRCODE='23514';
    END IF;
    recorded:=to_char(to_timestamp(result.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
        ||CASE WHEN result.recorded_at_nanoseconds=0 THEN 'Z'
            ELSE '.'||rtrim(lpad(result.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END;
    marker:='hrdc1:operation:'||NEW.operation_id::text||':capture:'||encode(NEW.capture_digest,'hex');
    SELECT * INTO audit FROM %1$I.audit_events WHERE sequence=NEW.audit_sequence;
    IF audit.actor COLLATE "C" IS DISTINCT FROM NEW.actor_email COLLATE "C"
        OR audit.action COLLATE "C" IS DISTINCT FROM 'hearing_derived_deadline.registered' COLLATE "C"
        OR audit.resource COLLATE "C" IS DISTINCT FROM marker COLLATE "C"
        OR audit.timestamp COLLATE "C" IS DISTINCT FROM recorded COLLATE "C" THEN
        RAISE EXCEPTION 'hearing derived deadline audit marker differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_hearing_derived_deadline_origins'::regclass
        AND tgname='hearing_derived_deadline_immutable') THEN
        CREATE TRIGGER hearing_derived_deadline_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
            ON case_hearing_derived_deadline_origins FOR EACH STATEMENT
            EXECUTE FUNCTION preserve_hearing_derived_deadline_history();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_hearing_derived_deadline_origins'::regclass
        AND tgname='hearing_derived_deadline_lock') THEN
        CREATE TRIGGER hearing_derived_deadline_lock BEFORE INSERT ON case_hearing_derived_deadline_origins
            FOR EACH STATEMENT EXECUTE FUNCTION lock_hearing_derived_deadline_history();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_hearing_derived_deadline_origins'::regclass
        AND tgname='hearing_derived_deadline_origin') THEN
        CREATE TRIGGER hearing_derived_deadline_origin BEFORE INSERT ON case_hearing_derived_deadline_origins
            FOR EACH ROW EXECUTE FUNCTION enforce_hearing_derived_deadline_origin();
    END IF;
END; $$;
REVOKE ALL ON FUNCTION preserve_hearing_derived_deadline_history(),lock_hearing_derived_deadline_history(),
    enforce_hearing_derived_deadline_origin() FROM PUBLIC;
