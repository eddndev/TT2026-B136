DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.owner_certificate_withdraw() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE actor RECORD; original RECORD; canonical BYTEA;
BEGIN
    SELECT r.* INTO original FROM %1$I.owner_certificate_registrations r
        WHERE r.binding_id=NEW.binding_id;
    SELECT email,role,active,revision,auth_generation INTO actor
        FROM %1$I.users WHERE id=original.owner_id FOR SHARE;
    IF original.binding_id IS NULL OR actor.active IS DISTINCT FROM TRUE
        OR actor.role IS DISTINCT FROM 'owner' OR actor.email IS DISTINCT FROM NEW.actor_email
        OR actor.revision IS DISTINCT FROM NEW.account_revision
        OR actor.auth_generation IS DISTINCT FROM NEW.auth_generation THEN
        RAISE EXCEPTION 'owner certificate withdrawal actor is not authorized' USING ERRCODE='42501';
    END IF;
    IF NEW.account_revision<original.account_revision OR NEW.auth_generation<original.auth_generation
        OR NEW.audit_sequence<=original.audit_sequence
        OR ROW(NEW.withdrawn_at_seconds,NEW.withdrawn_at_nanoseconds)
            <ROW(original.registered_at_seconds,original.registered_at_nanoseconds) THEN
        RAISE EXCEPTION 'owner certificate withdrawal precedes registration' USING ERRCODE='23514';
    END IF;
    canonical := convert_to('OWNCERT1','UTF8') || decode('0201','hex')
        || uuid_send(original.deployment_id) || original.root_fingerprint
        || substring(int8send(original.trust_revision) FROM 5 FOR 4)
        || uuid_send(original.owner_id) || int8send(NEW.account_revision) || int8send(NEW.auth_generation)
        || uuid_send(NEW.binding_id) || decode('0000000100000002','hex') || original.leaf_fingerprint;
    IF NEW.statement IS DISTINCT FROM canonical THEN
        RAISE EXCEPTION 'owner certificate withdrawal bytes differ' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence
        AND a.actor=NEW.actor_email AND a.action='identity.owner_certificate_withdrawn'
        AND a.resource='owner-certificate:' || NEW.binding_id::text || ':owner:' || original.owner_id::text
            || ':revision:2:statement:' || encode(NEW.statement_digest,'hex')
        AND a.timestamp_seconds=NEW.withdrawn_at_seconds AND a.timestamp_nanos=NEW.withdrawn_at_nanoseconds) THEN
        RAISE EXCEPTION 'owner certificate withdrawal audit differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['owner_certificate_registrations','owner_certificate_withdrawals'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='owner_certificate_lock') THEN
            EXECUTE format('CREATE TRIGGER owner_certificate_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION owner_certificate_lock()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='owner_certificate_immutable') THEN
            EXECUTE format('CREATE TRIGGER owner_certificate_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION owner_certificate_preserve()',tab);
        END IF;
    END LOOP;
END; $install$;
DROP TRIGGER IF EXISTS owner_certificate_register ON owner_certificate_registrations;
CREATE TRIGGER owner_certificate_register BEFORE INSERT ON owner_certificate_registrations
    FOR EACH ROW EXECUTE FUNCTION owner_certificate_register();
DROP TRIGGER IF EXISTS owner_certificate_withdraw ON owner_certificate_withdrawals;
CREATE TRIGGER owner_certificate_withdraw BEFORE INSERT ON owner_certificate_withdrawals
    FOR EACH ROW EXECUTE FUNCTION owner_certificate_withdraw();
REVOKE ALL ON FUNCTION owner_certificate_lock(),owner_certificate_preserve(),
    owner_certificate_register(),owner_certificate_withdraw() FROM PUBLIC;
