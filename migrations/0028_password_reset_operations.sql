DO $migration$
BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.password_reset_issue(
    p_id pg_catalog.uuid,p_email pg_catalog.text,p_digest pg_catalog.bytea,
    p_ttl pg_catalog.int8,p_capacity pg_catalog.int8)
RETURNS TABLE(id pg_catalog.uuid,email pg_catalog.text,expires_at pg_catalog.timestamptz)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $function$
DECLARE u %1$I.users%%ROWTYPE; at pg_catalog.timestamptz;
    expiry pg_catalog.timestamptz; live pg_catalog.int8;
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'reset requires read committed' USING ERRCODE='23514';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    IF p_id IS NULL OR p_id='00000000-0000-0000-0000-000000000000'::pg_catalog.uuid
        OR p_digest IS NULL OR pg_catalog.octet_length(p_digest)<>32 OR p_email IS NULL
        OR p_ttl IS NULL OR p_ttl<=0 OR p_capacity IS NULL
        OR p_capacity NOT BETWEEN 1 AND 4294967295 THEN
        RAISE EXCEPTION 'invalid reset issuance policy' USING ERRCODE='22023';
    END IF;
    SELECT * INTO u FROM %1$I.users r WHERE r.email=p_email FOR UPDATE;
    IF NOT FOUND OR NOT u.active THEN RETURN; END IF;
    at:=pg_catalog.clock_timestamp();
    IF NOT pg_catalog.isfinite(at) OR extract(epoch FROM at)<-62135596800
        OR p_ttl>=253402300800-extract(epoch FROM at) THEN
        RAISE EXCEPTION 'reset expiry outside supported range' USING ERRCODE='22023';
    END IF;
    expiry:=at+pg_catalog.make_interval(secs=>p_ttl::pg_catalog.float8);
    SELECT pg_catalog.count(*) INTO live FROM %1$I.password_reset_capabilities r
        WHERE r.user_id=u.id AND r.auth_generation=u.auth_generation
        AND r.consumed_at IS NULL AND r.cancelled_at IS NULL AND at<r.expires_at;
    IF live>=p_capacity THEN RETURN; END IF;
    INSERT INTO %1$I.password_reset_capabilities
        (id,digest,user_id,email,auth_generation,issued_at,expires_at)
        VALUES(p_id,p_digest,u.id,u.email,u.auth_generation,at,expiry);
    RETURN QUERY SELECT p_id,u.email,expiry;
END;
$function$;
$definition$,current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.password_reset_inspect(p_digest pg_catalog.bytea)
RETURNS TABLE(id pg_catalog.uuid,user_id pg_catalog.uuid,auth_generation pg_catalog.int8)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $function$
DECLARE at pg_catalog.timestamptz;
BEGIN
    IF p_digest IS NULL OR pg_catalog.octet_length(p_digest)<>32 THEN RETURN; END IF;
    at:=pg_catalog.clock_timestamp();
    RETURN QUERY SELECT r.id,r.user_id,r.auth_generation
        FROM %1$I.password_reset_capabilities r JOIN %1$I.users u ON u.id=r.user_id
        WHERE r.digest=p_digest AND r.consumed_at IS NULL AND r.cancelled_at IS NULL
        AND r.issued_at<=at AND at<r.expires_at AND u.active
        AND u.email=r.email AND u.auth_generation=r.auth_generation;
END;
$function$;
$definition$,current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.password_reset_cancel(p_id pg_catalog.uuid,p_digest pg_catalog.bytea)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $function$
DECLARE c %1$I.password_reset_capabilities%%ROWTYPE; at pg_catalog.timestamptz;
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'reset requires read committed' USING ERRCODE='23514';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    SELECT * INTO c FROM %1$I.password_reset_capabilities r
        WHERE r.id=p_id AND r.digest=p_digest FOR UPDATE;
    IF NOT FOUND OR c.consumed_at IS NOT NULL OR c.cancelled_at IS NOT NULL THEN RETURN; END IF;
    at:=pg_catalog.clock_timestamp();
    IF at<c.issued_at THEN RETURN; END IF;
    UPDATE %1$I.password_reset_capabilities SET cancelled_at=at WHERE id=c.id;
END;
$function$;
$definition$,current_schema());
END;
$migration$;
REVOKE ALL ON FUNCTION password_reset_issue(uuid,text,bytea,bigint,bigint),
    password_reset_inspect(bytea),password_reset_cancel(uuid,bytea) FROM PUBLIC;
