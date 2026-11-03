-- Keep Client accounts out of case memberships:
-- docs/adr/0086-bilateral-contract-signing.md. An existing Client membership
-- stops the migration; it is never deleted here.
DO $migration$
BEGIN
    IF EXISTS(SELECT 1 FROM case_memberships m JOIN users u ON u.id=m.user_id
            WHERE u.role='client') THEN
        RAISE EXCEPTION 'a client account is assigned to a case; remove that assignment before migrating'
            USING ERRCODE = '23514';
    END IF;
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.guard_client_membership() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $function$
BEGIN
    IF EXISTS(SELECT 1 FROM %1$I.users WHERE id=NEW.user_id AND role='client') THEN
        RAISE EXCEPTION 'client accounts are never case members' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_memberships'::regclass AND tgname='member_client_guard') THEN
        CREATE TRIGGER member_client_guard BEFORE INSERT OR UPDATE ON case_memberships
            FOR EACH ROW EXECUTE FUNCTION guard_client_membership();
    END IF;
END;
$migration$;
REVOKE ALL ON FUNCTION guard_client_membership() FROM PUBLIC;
