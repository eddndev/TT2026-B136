CREATE OR REPLACE FUNCTION owner_certificate_lock() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'owner certificate writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END;
$function$;

CREATE OR REPLACE FUNCTION owner_certificate_preserve() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'owner certificate history is immutable' USING ERRCODE='23514';
END;
$function$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.owner_certificate_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE actor RECORD; trust RECORD; canonical BYTEA;
BEGIN
    SELECT email,role,active,revision,auth_generation INTO actor
        FROM %1$I.users WHERE id=NEW.owner_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.role IS DISTINCT FROM 'owner'
        OR actor.email IS DISTINCT FROM NEW.actor_email
        OR actor.revision IS DISTINCT FROM NEW.account_revision
        OR actor.auth_generation IS DISTINCT FROM NEW.auth_generation THEN
        RAISE EXCEPTION 'owner certificate actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT t.*,a.root_fingerprint INTO trust
        FROM %1$I.participant_credential_trust_revisions t
        JOIN %1$I.participant_credential_authority a USING(deployment_id)
        ORDER BY t.revision DESC LIMIT 1;
    IF trust.deployment_id IS DISTINCT FROM NEW.deployment_id
        OR trust.revision IS DISTINCT FROM NEW.trust_revision
        OR trust.root_fingerprint IS DISTINCT FROM NEW.root_fingerprint
        OR NEW.valid_from IS DISTINCT FROM greatest(NEW.certificate_not_before,trust.valid_from)
        OR NEW.valid_until IS DISTINCT FROM least(NEW.certificate_not_after,trust.valid_until)
        OR ROW(NEW.registered_at_seconds,NEW.registered_at_nanoseconds)
            <ROW(trust.published_at_seconds,trust.published_at_nanoseconds) THEN
        RAISE EXCEPTION 'owner certificate current trust differs' USING ERRCODE='23514';
    END IF;
    canonical := convert_to('OWNCERT1','UTF8') || decode('0101','hex')
        || uuid_send(NEW.deployment_id) || NEW.root_fingerprint
        || substring(int8send(NEW.trust_revision) FROM 5 FOR 4)
        || uuid_send(NEW.owner_id) || int8send(NEW.account_revision) || int8send(NEW.auth_generation)
        || uuid_send(NEW.binding_id) || decode('0000000000000001','hex') || NEW.leaf_fingerprint;
    IF NEW.statement IS DISTINCT FROM canonical THEN
        RAISE EXCEPTION 'owner certificate registration bytes differ' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.owner_certificate_registrations r
            WHERE r.leaf_fingerprint=NEW.leaf_fingerprint AND r.owner_id<>NEW.owner_id)
        OR EXISTS(SELECT 1 FROM %1$I.owner_certificate_registrations r
            WHERE r.owner_id=NEW.owner_id AND NOT EXISTS(
                SELECT 1 FROM %1$I.owner_certificate_withdrawals w WHERE w.binding_id=r.binding_id)) THEN
        RAISE EXCEPTION 'owner certificate claim already exists' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.owner_certificate_withdrawals w
            JOIN %1$I.owner_certificate_registrations r USING(binding_id)
            WHERE r.owner_id=NEW.owner_id AND (w.audit_sequence>=NEW.audit_sequence
                OR w.account_revision>NEW.account_revision OR w.auth_generation>NEW.auth_generation
                OR ROW(w.withdrawn_at_seconds,w.withdrawn_at_nanoseconds)
                    >ROW(NEW.registered_at_seconds,NEW.registered_at_nanoseconds))) THEN
        RAISE EXCEPTION 'owner certificate registration precedes history' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence
        AND a.actor=NEW.actor_email AND a.action='identity.owner_certificate_registered'
        AND a.resource='owner-certificate:' || NEW.binding_id::text || ':owner:' || NEW.owner_id::text
            || ':revision:1:statement:' || encode(NEW.statement_digest,'hex')
        AND a.timestamp_seconds=NEW.registered_at_seconds AND a.timestamp_nanos=NEW.registered_at_nanoseconds) THEN
        RAISE EXCEPTION 'owner certificate registration audit differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;
