DO $migration$
BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.guard_member_access() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
    IF current_user::pg_catalog.regrole::pg_catalog.oid=(SELECT p.proowner FROM pg_catalog.pg_proc p
        WHERE p.oid=pg_catalog.to_regprocedure(pg_catalog.quote_ident(TG_TABLE_SCHEMA)
            ||'.password_reset_consume(uuid,bytea,uuid,bigint,text)'))
        AND NEW.auth_generation::pg_catalog.numeric=OLD.auth_generation::pg_catalog.numeric+1
        AND NEW.password_hash IS DISTINCT FROM OLD.password_hash THEN
        IF NOT OLD.active OR OLD.revision=9223372036854775807
            OR OLD.auth_generation=9223372036854775807
            OR NEW.revision::pg_catalog.numeric<>OLD.revision::pg_catalog.numeric+1
            OR ROW(NEW.id,NEW.email,NEW.role,NEW.active,NEW.protected_totp_secret,
                NEW.recovery_codes,NEW.created_at) IS DISTINCT FROM
                ROW(OLD.id,OLD.email,OLD.role,OLD.active,OLD.protected_totp_secret,
                    OLD.recovery_codes,OLD.created_at)
            OR pg_catalog.octet_length(NEW.password_hash)=0 OR NOT pg_catalog.isfinite(NEW.updated_at)
            OR extract(epoch FROM NEW.updated_at)<-62135596800
            OR extract(epoch FROM NEW.updated_at)>=253402300800 THEN
            RAISE EXCEPTION 'invalid password reset transition' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.created_at IS DISTINCT FROM OLD.created_at
        OR NEW.password_hash IS DISTINCT FROM OLD.password_hash
        OR NEW.protected_totp_secret IS DISTINCT FROM OLD.protected_totp_secret THEN
        RAISE EXCEPTION 'account identity and credentials are immutable' USING ERRCODE = '23514';
    END IF;
    IF NEW.role IS DISTINCT FROM OLD.role OR NEW.active IS DISTINCT FROM OLD.active THEN
        IF OLD.revision=9223372036854775807 OR OLD.auth_generation=9223372036854775807
            OR NEW.revision<>OLD.revision+1 OR NEW.auth_generation<>OLD.auth_generation+1
            OR NEW.recovery_codes IS DISTINCT FROM OLD.recovery_codes THEN
            RAISE EXCEPTION 'access changes require fresh revision and generation' USING ERRCODE = '23514';
        END IF;
        IF OLD.active AND OLD.role='owner' AND (NOT NEW.active OR NEW.role<>'owner')
            AND NOT EXISTS(SELECT 1 FROM %1$I.users WHERE id<>OLD.id AND active AND role='owner') THEN
            RAISE EXCEPTION 'at least one active owner must remain' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF NEW.auth_generation<>OLD.auth_generation OR NEW.revision<OLD.revision
            OR NEW.revision::pg_catalog.numeric>OLD.revision::pg_catalog.numeric+1
            OR (NEW.recovery_codes IS DISTINCT FROM OLD.recovery_codes AND NEW.revision::pg_catalog.numeric<>OLD.revision::pg_catalog.numeric+1) THEN
            RAISE EXCEPTION 'invalid account counter transition' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.password_reset_audit_receipt() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF NEW.consumed_at IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence
        AND a.actor='password-reset' AND a.action='identity.password_reset'
        AND a.resource='user:'||NEW.user_id::pg_catalog.text||':revision:'||NEW.consumed_revision::pg_catalog.text
            ||':generation:'||NEW.consumed_generation::pg_catalog.text
        AND a.timestamp::pg_catalog.timestamptz=NEW.consumed_at) THEN
        RAISE EXCEPTION 'password reset requires matching audit' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$function$;
$definition$,current_schema());
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='password_reset_capabilities'::regclass
        AND tgname='password_reset_requires_audit') THEN
        CREATE CONSTRAINT TRIGGER password_reset_requires_audit
            AFTER UPDATE ON password_reset_capabilities DEFERRABLE INITIALLY DEFERRED
            FOR EACH ROW EXECUTE FUNCTION password_reset_audit_receipt();
    END IF;
END;
$migration$;
REVOKE ALL ON FUNCTION guard_member_access(),password_reset_audit_receipt() FROM PUBLIC;
