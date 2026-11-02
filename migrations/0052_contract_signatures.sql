-- Contract party signatures, completion, notifications and the version guard:
-- docs/adr/0086-bilateral-contract-signing.md. A signature follows its own
-- event in the same transaction; the completion follows the second signature.
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.contract_signature_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE request RECORD; event RECORD; actor RECORD; binding RECORD; trust RECORD; first RECORD;
    sealed BOOLEAN;
    signature_digest BYTEA := decode(repeat('00',32),'hex'); token_digest BYTEA := signature_digest;
BEGIN
    SELECT * INTO request FROM %1$I.contract_signature_requests WHERE request_id=NEW.request_id;
    SELECT * INTO event FROM %1$I.contract_events WHERE operation_id=NEW.operation_id;
    IF event.request_id IS DISTINCT FROM NEW.request_id
        OR event.kind IS DISTINCT FROM (CASE NEW."position" WHEN 1 THEN 'client_signed' ELSE 'completed' END)
        OR event.actor_id IS DISTINCT FROM NEW.signer_id OR event.actor_label IS DISTINCT FROM NEW.actor_email
        OR ROW(event.at_seconds,event.at_nanoseconds)
            IS DISTINCT FROM ROW(NEW.accepted_at_seconds,NEW.accepted_at_nanoseconds) THEN
        RAISE EXCEPTION 'contract signature is out of turn' USING ERRCODE='23514';
    END IF;
    IF (NEW."position"=1 AND ROW(NEW.signer_id,NEW.binding_id,NEW.leaf_fingerprint) IS DISTINCT FROM
            ROW(request.client_id,request.client_binding_id,request.client_leaf))
        OR (NEW."position"=2 AND ROW(NEW.signer_id,NEW.binding_id,NEW.leaf_fingerprint) IS DISTINCT FROM
            ROW(request.firm_signer_id,request.firm_binding_id,request.firm_leaf))
        OR NEW.deployment_id IS DISTINCT FROM request.deployment_id THEN
        RAISE EXCEPTION 'contract signer is not designated' USING ERRCODE='42501';
    END IF;
    IF NEW.accepted_at_seconds>=request.expires_at_seconds
        OR ROW(NEW.accepted_at_seconds,NEW.accepted_at_nanoseconds)
            <ROW(request.created_at_seconds,request.created_at_nanoseconds) THEN
        RAISE EXCEPTION 'contract request has expired' USING ERRCODE='23514';
    END IF;
    SELECT email,role,active,revision,auth_generation INTO actor
        FROM %1$I.users WHERE id=NEW.signer_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.actor_email
        OR actor.revision IS DISTINCT FROM NEW.account_revision
        OR actor.auth_generation IS DISTINCT FROM NEW.auth_generation
        OR (NEW."position"=1 AND actor.role IS DISTINCT FROM 'client')
        OR (NEW."position"=2 AND NOT coalesce(actor.role='owner' OR (actor.role='litigator'
            AND EXISTS(SELECT 1 FROM %1$I.case_memberships
                WHERE case_id=request.case_id AND user_id=NEW.signer_id)),FALSE))
        OR coalesce((SELECT administrative_status='closed' FROM %1$I.case_administration_revisions
            WHERE case_id=request.case_id ORDER BY revision DESC LIMIT 1),FALSE) THEN
        RAISE EXCEPTION 'contract signer is not currently authorized' USING ERRCODE='42501';
    END IF;
    IF EXISTS(SELECT 1 FROM %1$I.documents WHERE id=request.document_id
        AND version>request.document_version) THEN
        RAISE EXCEPTION 'contract version is superseded' USING ERRCODE='23514';
    END IF;
    -- The version guard reads the database clock and this row carries the
    -- application's, so a seal admitted after expires_at must stop a
    -- signature accepted before it.
    SELECT evidence IS NOT NULL INTO sealed FROM %1$I.documents
        WHERE id=request.document_id AND version=request.document_version FOR SHARE;
    IF sealed IS NOT FALSE THEN
        RAISE EXCEPTION 'contract version is frozen' USING ERRCODE='23514';
    END IF;
    SELECT * INTO binding FROM %1$I.document_signing_bindings WHERE binding_id=NEW.binding_id;
    IF binding.deployment_id IS DISTINCT FROM NEW.deployment_id
        OR binding.root_fingerprint IS DISTINCT FROM NEW.root_fingerprint
        OR binding.account_revision>NEW.account_revision OR binding.auth_generation>NEW.auth_generation
        OR binding.trust_revision>NEW.trust_revision
        OR ROW(binding.registered_at_seconds,binding.registered_at_nanoseconds)
            >ROW(NEW.accepted_at_seconds,NEW.accepted_at_nanoseconds)
        OR EXISTS(SELECT 1 FROM %1$I.document_signing_withdrawals WHERE binding_id=NEW.binding_id) THEN
        RAISE EXCEPTION 'contract signing binding is inactive or inconsistent' USING ERRCODE='23514';
    END IF;
    SELECT t.*,a.root_fingerprint INTO trust FROM %1$I.participant_credential_trust_revisions t
        JOIN %1$I.participant_credential_authority a USING(deployment_id)
        ORDER BY t.revision DESC LIMIT 1;
    IF trust.deployment_id IS DISTINCT FROM NEW.deployment_id
        OR trust.revision IS DISTINCT FROM NEW.trust_revision
        OR trust.root_fingerprint IS DISTINCT FROM NEW.root_fingerprint
        OR NEW.valid_from IS DISTINCT FROM greatest(binding.certificate_not_before,trust.valid_from)
        OR NEW.valid_until IS DISTINCT FROM least(binding.certificate_not_after,trust.valid_until)
        OR ROW(NEW.accepted_at_seconds,NEW.accepted_at_nanoseconds)
            <ROW(trust.published_at_seconds,trust.published_at_nanoseconds) THEN
        RAISE EXCEPTION 'contract signing current trust differs' USING ERRCODE='23514';
    END IF;
    IF NEW."position"=2 THEN
        SELECT * INTO first FROM %1$I.contract_party_signatures
            WHERE request_id=NEW.request_id AND "position"=1;
        IF first.root_fingerprint IS DISTINCT FROM NEW.root_fingerprint
            OR first.deployment_id IS DISTINCT FROM NEW.deployment_id THEN
            RAISE EXCEPTION 'contract parties differ in root' USING ERRCODE='23514';
        END IF;
        signature_digest := sha256(first.signature);
        token_digest := sha256(first.timestamp_token);
    END IF;
    IF NEW.statement IS DISTINCT FROM %1$I.contract_party_bytes(NEW.deployment_id,NEW.root_fingerprint,
        NEW.trust_revision,NEW.signer_id,NEW.account_revision,NEW.auth_generation,NEW.binding_id,
        NEW.leaf_fingerprint,NEW.operation_id,NEW.request_id,request.statement_digest,
        request.content_digest,NEW."position",signature_digest,token_digest) THEN
        RAISE EXCEPTION 'contract statement bytes differ' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.contract_completion_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE second RECORD;
BEGIN
    SELECT * INTO second FROM %1$I.contract_party_signatures
        WHERE request_id=NEW.request_id AND "position"=2;
    IF second.operation_id IS NULL
        OR ROW(second.accepted_at_seconds,second.accepted_at_nanoseconds)
            IS DISTINCT FROM ROW(NEW.completed_at_seconds,NEW.completed_at_nanoseconds) THEN
        RAISE EXCEPTION 'contract completion differs from its signature' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.contract_completion_pair() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE paired BOOLEAN := TRUE;
BEGIN
    IF TG_TABLE_NAME='contracts' THEN
        paired := EXISTS(SELECT 1 FROM %1$I.contract_events WHERE contract_id=NEW.id AND revision=1);
    ELSIF TG_TABLE_NAME='contract_signature_requests' THEN
        paired := EXISTS(SELECT 1 FROM %1$I.contract_events
            WHERE request_id=NEW.request_id AND kind='requested');
    ELSIF NEW.kind='client_signed' THEN
        paired := EXISTS(SELECT 1 FROM %1$I.contract_party_signatures
            WHERE operation_id=NEW.operation_id AND "position"=1);
    ELSIF NEW.kind='completed' THEN
        paired := EXISTS(SELECT 1 FROM %1$I.contract_party_signatures
                WHERE operation_id=NEW.operation_id AND "position"=2)
            AND EXISTS(SELECT 1 FROM %1$I.contract_completions WHERE request_id=NEW.request_id);
    END IF;
    IF NOT paired THEN
        RAISE EXCEPTION 'contract record lacks its paired rows' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.contract_notification_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE event RECORD; request RECORD;
BEGIN
    SELECT * INTO event FROM %1$I.contract_events
        WHERE contract_id=NEW.contract_id AND revision=NEW.event_revision;
    SELECT * INTO request FROM %1$I.contract_signature_requests WHERE request_id=NEW.request_id;
    IF event.request_id IS DISTINCT FROM NEW.request_id OR event.kind IS DISTINCT FROM NEW.kind
        OR event.at_seconds IS DISTINCT FROM NEW.created_at_seconds
        OR NEW.delivery_state<>'pending' OR NEW.delivery_generation<>0 OR NEW.lease_id IS NOT NULL
        OR NEW.provider_deadline IS NOT NULL OR NEW.retry_at IS NOT NULL
        OR NEW.message_to IS DISTINCT FROM (SELECT email FROM %1$I.users WHERE id=NEW.recipient_id)
        OR NOT (CASE NEW.kind
            WHEN 'requested' THEN ARRAY[request.client_id]
            WHEN 'rejected' THEN ARRAY[request.requester_id,request.firm_signer_id]
            WHEN 'declined' THEN ARRAY[request.client_id,request.requester_id]
            WHEN 'cancelled' THEN ARRAY[request.client_id,request.firm_signer_id]
            ELSE ARRAY[request.client_id,request.firm_signer_id,request.requester_id] END
            @> ARRAY[NEW.recipient_id]) THEN
        RAISE EXCEPTION 'contract notification differs from its event' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

CREATE OR REPLACE FUNCTION contract_notification_guard() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF ROW(NEW.id,NEW.request_id,NEW.contract_id,NEW.event_revision,NEW.recipient_id,NEW.kind,
            NEW.message_from,NEW.message_to,NEW.subject,NEW.message_text,NEW.created_at_seconds)
        IS DISTINCT FROM ROW(OLD.id,OLD.request_id,OLD.contract_id,OLD.event_revision,OLD.recipient_id,
            OLD.kind,OLD.message_from,OLD.message_to,OLD.subject,OLD.message_text,OLD.created_at_seconds)
        OR NEW.delivery_generation<OLD.delivery_generation THEN
        RAISE EXCEPTION 'contract notification content is immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.contract_version_guard() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF NEW.evidence IS NOT NULL AND EXISTS(SELECT 1 FROM %1$I.contract_signature_requests r
        WHERE r.document_id=NEW.id AND r.document_version=NEW.version AND (
            EXISTS(SELECT 1 FROM %1$I.contract_events e
                WHERE e.request_id=r.request_id AND e.kind='completed')
            OR (extract(epoch FROM clock_timestamp())<r.expires_at_seconds
                AND NOT EXISTS(SELECT 1 FROM %1$I.contract_events e WHERE e.request_id=r.request_id
                    AND e.kind IN ('rejected','declined','cancelled','expired'))))) THEN
        RAISE EXCEPTION 'contract version is frozen' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['contracts','contract_signature_requests','contract_events',
        'contract_party_signatures','contract_completions','contract_notifications'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='contract_lock') THEN
            EXECUTE format('CREATE TRIGGER contract_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION contract_lock()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='contract_immutable') THEN
            EXECUTE format('CREATE TRIGGER contract_immutable BEFORE %s ON %I FOR EACH STATEMENT EXECUTE FUNCTION contract_preserve()',
                CASE tab WHEN 'contract_notifications' THEN 'DELETE OR TRUNCATE'
                    ELSE 'UPDATE OR DELETE OR TRUNCATE' END,tab);
        END IF;
    END LOOP;
END; $install$;
DROP TRIGGER IF EXISTS contract_event_register ON contract_events;
CREATE TRIGGER contract_event_register BEFORE INSERT ON contract_events
    FOR EACH ROW EXECUTE FUNCTION contract_event_register();
DROP TRIGGER IF EXISTS contract_request_register ON contract_signature_requests;
CREATE TRIGGER contract_request_register BEFORE INSERT ON contract_signature_requests
    FOR EACH ROW EXECUTE FUNCTION contract_request_register();
DROP TRIGGER IF EXISTS contract_signature_register ON contract_party_signatures;
CREATE TRIGGER contract_signature_register BEFORE INSERT ON contract_party_signatures
    FOR EACH ROW EXECUTE FUNCTION contract_signature_register();
DROP TRIGGER IF EXISTS contract_completion_register ON contract_completions;
CREATE TRIGGER contract_completion_register BEFORE INSERT ON contract_completions
    FOR EACH ROW EXECUTE FUNCTION contract_completion_register();
DROP TRIGGER IF EXISTS contract_notification_register ON contract_notifications;
CREATE TRIGGER contract_notification_register BEFORE INSERT ON contract_notifications
    FOR EACH ROW EXECUTE FUNCTION contract_notification_register();
DROP TRIGGER IF EXISTS contract_notification_guard ON contract_notifications;
CREATE TRIGGER contract_notification_guard BEFORE UPDATE ON contract_notifications
    FOR EACH ROW EXECUTE FUNCTION contract_notification_guard();
DROP TRIGGER IF EXISTS contract_root_pair ON contracts;
CREATE CONSTRAINT TRIGGER contract_root_pair AFTER INSERT ON contracts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION contract_completion_pair();
DROP TRIGGER IF EXISTS contract_request_pair ON contract_signature_requests;
CREATE CONSTRAINT TRIGGER contract_request_pair AFTER INSERT ON contract_signature_requests
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION contract_completion_pair();
DROP TRIGGER IF EXISTS contract_completion_pair ON contract_events;
CREATE CONSTRAINT TRIGGER contract_completion_pair AFTER INSERT ON contract_events
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION contract_completion_pair();
DROP TRIGGER IF EXISTS contract_version_guard ON documents;
CREATE TRIGGER contract_version_guard BEFORE UPDATE OF evidence ON documents
    FOR EACH ROW EXECUTE FUNCTION contract_version_guard();
REVOKE ALL ON FUNCTION contract_lock(),contract_preserve(),contract_event_register(),
    contract_request_register(),contract_signature_register(),contract_completion_register(),
    contract_completion_pair(),contract_notification_register(),contract_notification_guard(),
    contract_version_guard() FROM PUBLIC;
