DO $migration$
BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.password_reset_inventory_valid() RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $function$
BEGIN
    RETURN NOT EXISTS(SELECT 1 FROM %1$I.password_reset_capabilities r
        LEFT JOIN %1$I.users u ON u.id=r.user_id
        LEFT JOIN %1$I.audit_events a ON a.sequence=r.audit_sequence
        WHERE u.id IS NULL OR r.auth_generation>u.auth_generation
        OR r.auth_generation<0 OR pg_catalog.octet_length(r.digest)<>32
        OR r.id='00000000-0000-0000-0000-000000000000'::pg_catalog.uuid
        OR NOT pg_catalog.isfinite(r.issued_at) OR NOT pg_catalog.isfinite(r.expires_at)
        OR extract(epoch FROM r.issued_at)<-62135596800
        OR extract(epoch FROM r.expires_at)>=253402300800 OR r.expires_at<=r.issued_at
        OR (r.cancelled_at IS NOT NULL AND (NOT pg_catalog.isfinite(r.cancelled_at)
            OR r.cancelled_at<r.issued_at OR extract(epoch FROM r.cancelled_at)>=253402300800))
        OR (r.consumed_at IS NULL AND (r.consumed_revision IS NOT NULL
            OR r.consumed_generation IS NOT NULL OR r.audit_sequence IS NOT NULL))
        OR (r.consumed_at IS NOT NULL AND (r.cancelled_at IS NOT NULL
            OR r.consumed_at<r.issued_at OR r.consumed_at>=r.expires_at
            OR r.consumed_revision IS NULL OR r.consumed_revision<=0
            OR r.consumed_generation IS NULL OR r.consumed_generation<=0
            OR r.consumed_generation::pg_catalog.numeric<>r.auth_generation::pg_catalog.numeric+1
            OR r.consumed_generation>r.consumed_revision
            OR r.consumed_revision>u.revision OR r.consumed_generation>u.auth_generation
            OR r.audit_sequence IS NULL OR a.sequence IS NULL
            OR a.actor<>'password-reset' OR a.action<>'identity.password_reset'
            OR a.resource<>'user:'||r.user_id::pg_catalog.text||':revision:'||r.consumed_revision::pg_catalog.text
                ||':generation:'||r.consumed_generation::pg_catalog.text
            OR a.timestamp::pg_catalog.timestamptz<>r.consumed_at)));
END;
$function$;
$definition$,current_schema());
END;
$migration$;
REVOKE ALL ON FUNCTION password_reset_inventory_valid() FROM PUBLIC;
