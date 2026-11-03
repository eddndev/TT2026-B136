-- Admit every party of a bilateral contract to a document-signing binding:
-- docs/adr/0086-bilateral-contract-signing.md. Individual signatures keep
-- requiring an active Litigator in their own receipt guard.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.document_signing_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE actor RECORD; trust RECORD; canonical BYTEA;
BEGIN
    SELECT email,role,active,revision,auth_generation INTO actor
        FROM %1$I.users WHERE id=NEW.signer_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE
        OR actor.role IS NULL OR actor.role NOT IN ('litigator','owner','client')
        OR actor.email IS DISTINCT FROM NEW.actor_email
        OR actor.revision IS DISTINCT FROM NEW.account_revision
        OR actor.auth_generation IS DISTINCT FROM NEW.auth_generation THEN
        RAISE EXCEPTION 'document signing actor is not currently authorized' USING ERRCODE='42501';
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
        RAISE EXCEPTION 'document signing current trust differs' USING ERRCODE='23514';
    END IF;
    canonical := convert_to('DOCBIND1','UTF8') || decode('0101','hex')
        || uuid_send(NEW.deployment_id) || NEW.root_fingerprint
        || substring(int8send(NEW.trust_revision) FROM 5 FOR 4)
        || uuid_send(NEW.signer_id) || int8send(NEW.account_revision) || int8send(NEW.auth_generation)
        || uuid_send(NEW.binding_id) || NEW.leaf_fingerprint;
    IF NEW.statement IS DISTINCT FROM canonical THEN
        RAISE EXCEPTION 'document signing registration bytes differ' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.document_signing_bindings r
            WHERE r.leaf_fingerprint=NEW.leaf_fingerprint AND r.signer_id<>NEW.signer_id)
        OR EXISTS(SELECT 1 FROM %1$I.document_signing_bindings r
            WHERE r.signer_id=NEW.signer_id AND NOT EXISTS(
                SELECT 1 FROM %1$I.document_signing_withdrawals w WHERE w.binding_id=r.binding_id)) THEN
        RAISE EXCEPTION 'document signing claim already exists' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.document_signing_withdrawals w
            JOIN %1$I.document_signing_bindings r USING(binding_id)
            WHERE r.signer_id=NEW.signer_id AND (w.audit_sequence>=NEW.audit_sequence
                OR w.account_revision>NEW.account_revision OR w.auth_generation>NEW.auth_generation
                OR ROW(w.withdrawn_at_seconds,w.withdrawn_at_nanoseconds)
                    >ROW(NEW.registered_at_seconds,NEW.registered_at_nanoseconds))) THEN
        RAISE EXCEPTION 'document signing registration precedes history' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence
        AND a.actor=NEW.actor_email AND a.action='document.signer_bound'
        AND a.resource='document-signer:' || NEW.binding_id::text || ':signer:' || NEW.signer_id::text
            || ':revision:1:statement:' || encode(NEW.statement_digest,'hex')
        AND a.timestamp_seconds=NEW.registered_at_seconds AND a.timestamp_nanos=NEW.registered_at_nanoseconds) THEN
        RAISE EXCEPTION 'document signing registration audit differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.document_signing_withdraw() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE actor RECORD; original RECORD;
BEGIN
    SELECT r.* INTO original FROM %1$I.document_signing_bindings r
        WHERE r.binding_id=NEW.binding_id;
    SELECT email,role,active,revision,auth_generation INTO actor
        FROM %1$I.users WHERE id=original.signer_id FOR SHARE;
    IF original.binding_id IS NULL OR actor.active IS DISTINCT FROM TRUE
        OR actor.role IS NULL OR actor.role NOT IN ('litigator','owner','client')
        OR actor.email IS DISTINCT FROM NEW.actor_email
        OR actor.revision IS DISTINCT FROM NEW.account_revision
        OR actor.auth_generation IS DISTINCT FROM NEW.auth_generation THEN
        RAISE EXCEPTION 'document signing withdrawal actor is not authorized' USING ERRCODE='42501';
    END IF;
    IF NEW.account_revision<original.account_revision OR NEW.auth_generation<original.auth_generation
        OR NEW.audit_sequence<=original.audit_sequence
        OR ROW(NEW.withdrawn_at_seconds,NEW.withdrawn_at_nanoseconds)
            <ROW(original.registered_at_seconds,original.registered_at_nanoseconds) THEN
        RAISE EXCEPTION 'document signing withdrawal precedes registration' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence
        AND a.actor=NEW.actor_email AND a.action='document.signer_withdrawn'
        AND a.resource='document-signer:' || NEW.binding_id::text || ':signer:' || original.signer_id::text
            || ':revision:2:statement:' || encode(original.statement_digest,'hex')
        AND a.timestamp_seconds=NEW.withdrawn_at_seconds AND a.timestamp_nanos=NEW.withdrawn_at_nanoseconds) THEN
        RAISE EXCEPTION 'document signing withdrawal audit differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;
