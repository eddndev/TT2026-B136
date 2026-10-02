DO $migration$
BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.password_reset_consume(
    p_id pg_catalog.uuid,p_digest pg_catalog.bytea,p_user pg_catalog.uuid,
    p_generation pg_catalog.int8,p_hash pg_catalog.text)
RETURNS TABLE(reset_at pg_catalog.timestamptz,next_audit pg_catalog.int8,
    new_revision pg_catalog.int8,new_generation pg_catalog.int8)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $function$
DECLARE c %1$I.password_reset_capabilities%%ROWTYPE; u %1$I.users%%ROWTYPE;
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'reset requires read committed' USING ERRCODE='23514';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    IF p_id IS NULL OR p_digest IS NULL OR pg_catalog.octet_length(p_digest)<>32
        OR p_user IS NULL OR p_generation IS NULL OR p_generation<0
        OR p_hash IS NULL OR pg_catalog.octet_length(p_hash)=0 THEN RETURN; END IF;
    SELECT * INTO c FROM %1$I.password_reset_capabilities r
        WHERE r.id=p_id AND r.digest=p_digest FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT * INTO u FROM %1$I.users r WHERE r.id=c.user_id FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    reset_at:=pg_catalog.clock_timestamp();
    IF c.consumed_at IS NOT NULL OR c.cancelled_at IS NOT NULL
        OR reset_at<c.issued_at OR reset_at>=c.expires_at OR NOT u.active
        OR u.id<>p_user OR u.email<>c.email OR c.auth_generation<>p_generation
        OR u.auth_generation<>c.auth_generation OR u.password_hash=p_hash
        OR u.revision=9223372036854775807
        OR u.auth_generation=9223372036854775807 THEN RETURN; END IF;
    SELECT COALESCE(pg_catalog.max(a.sequence)::pg_catalog.numeric+1,0) INTO next_audit
        FROM %1$I.audit_events a;
    new_revision:=u.revision+1; new_generation:=u.auth_generation+1;
    UPDATE %1$I.users SET password_hash=p_hash,revision=new_revision,
        auth_generation=new_generation,updated_at=reset_at WHERE id=u.id;
    UPDATE %1$I.password_reset_capabilities SET consumed_at=reset_at,
        consumed_revision=new_revision,consumed_generation=new_generation,
        audit_sequence=next_audit WHERE id=c.id;
    RETURN NEXT;
END;
$function$;
$definition$,current_schema());
END;
$migration$;
REVOKE ALL ON FUNCTION password_reset_consume(uuid,bytea,uuid,bigint,text) FROM PUBLIC;
