ALTER TABLE users ADD COLUMN IF NOT EXISTS auth_generation BIGINT NOT NULL DEFAULT 0;
DO $migration$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid='users'::regclass AND conname='member_auth_generation_check') THEN
        ALTER TABLE users ADD CONSTRAINT member_auth_generation_check CHECK(auth_generation >= 0);
    END IF;
END;
$migration$;

CREATE OR REPLACE FUNCTION lock_member_writes() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'member writes require read committed' USING ERRCODE = '23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END;
$function$;

CREATE OR REPLACE FUNCTION preserve_member_accounts() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'member accounts cannot be deleted' USING ERRCODE = '23514';
END;
$function$;

DO $migration$
BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.guard_member_access() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
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
            OR NEW.revision::numeric>OLD.revision::numeric+1
            OR (NEW.recovery_codes IS DISTINCT FROM OLD.recovery_codes AND NEW.revision::numeric<>OLD.revision::numeric+1) THEN
            RAISE EXCEPTION 'invalid account counter transition' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='users'::regclass AND tgname='member_write_lock') THEN
        CREATE TRIGGER member_write_lock BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON users
            FOR EACH STATEMENT EXECUTE FUNCTION lock_member_writes();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_memberships'::regclass AND tgname='member_write_lock') THEN
        CREATE TRIGGER member_write_lock BEFORE INSERT OR UPDATE OR DELETE OR TRUNCATE ON case_memberships
            FOR EACH STATEMENT EXECUTE FUNCTION lock_member_writes();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='users'::regclass AND tgname='member_access_guard') THEN
        CREATE TRIGGER member_access_guard BEFORE UPDATE ON users
            FOR EACH ROW EXECUTE FUNCTION guard_member_access();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='users'::regclass AND tgname='member_no_delete') THEN
        CREATE TRIGGER member_no_delete BEFORE DELETE OR TRUNCATE ON users
            FOR EACH STATEMENT EXECUTE FUNCTION preserve_member_accounts();
    END IF;
END;
$migration$;
REVOKE ALL ON users,case_memberships FROM PUBLIC;
REVOKE ALL ON FUNCTION lock_member_writes(),guard_member_access(),preserve_member_accounts() FROM PUBLIC;
