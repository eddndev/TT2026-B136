-- Contract events and signature requests: docs/adr/0086-bilateral-contract-signing.md.
-- Every contract write takes the document signing lock after the audited
-- mutation lock, so requests, signatures, bindings and seals serialize.
CREATE OR REPLACE FUNCTION contract_lock() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'contract writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END;
$function$;

CREATE OR REPLACE FUNCTION contract_preserve() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'contract history is immutable' USING ERRCODE='23514';
END;
$function$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.contract_event_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE root RECORD; previous RECORD; request RECORD; actor RECORD; staff BOOLEAN; closed BOOLEAN;
BEGIN
    SELECT * INTO root FROM %1$I.contracts WHERE id=NEW.contract_id;
    SELECT * INTO previous FROM %1$I.contract_events WHERE contract_id=NEW.contract_id
        ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM coalesce(previous.revision,0)+1 THEN
        RAISE EXCEPTION 'contract revision conflict' USING ERRCODE='40001';
    END IF;
    IF previous.revision IS NOT NULL AND (NEW.audit_sequence<=previous.audit_sequence
        OR ROW(NEW.at_seconds,NEW.at_nanoseconds)<ROW(previous.at_seconds,previous.at_nanoseconds)) THEN
        RAISE EXCEPTION 'contract event precedes its history' USING ERRCODE='23514';
    END IF;
    closed := coalesce((SELECT administrative_status='closed' FROM %1$I.case_administration_revisions
        WHERE case_id=root.case_id ORDER BY revision DESC LIMIT 1),FALSE);
    IF NEW.actor_id IS NOT NULL THEN
        SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.actor_id FOR SHARE;
        staff := actor.role='owner' OR (actor.role='litigator' AND EXISTS(
            SELECT 1 FROM %1$I.case_memberships WHERE case_id=root.case_id AND user_id=NEW.actor_id));
        IF actor.active IS DISTINCT FROM TRUE OR actor.email IS DISTINCT FROM NEW.actor_label OR closed THEN
            RAISE EXCEPTION 'contract actor is not currently authorized' USING ERRCODE='42501';
        END IF;
    END IF;
    IF NEW.kind='created' THEN
        IF staff IS NOT TRUE THEN
            RAISE EXCEPTION 'contract actor is not currently authorized' USING ERRCODE='42501';
        END IF;
        IF root.created_by IS DISTINCT FROM NEW.actor_id OR root.created_by_email IS DISTINCT FROM NEW.actor_label
            OR ROW(root.created_at_seconds,root.created_at_nanoseconds)
                IS DISTINCT FROM ROW(NEW.at_seconds,NEW.at_nanoseconds) THEN
            RAISE EXCEPTION 'contract creation differs from its root' USING ERRCODE='23514';
        END IF;
    ELSIF NEW.kind IN ('requested','abandoned') THEN
        IF previous.kind IN ('requested','client_signed') THEN
            SELECT * INTO request FROM %1$I.contract_signature_requests WHERE request_id=previous.request_id;
            IF NEW.at_seconds<request.expires_at_seconds THEN
                RAISE EXCEPTION 'contract request is open' USING ERRCODE='23514';
            END IF;
            RAISE EXCEPTION 'contract request must expire first' USING ERRCODE='23514';
        ELSIF previous.kind IS DISTINCT FROM 'created' AND previous.kind NOT IN
            ('rejected','declined','cancelled','expired') THEN
            RAISE EXCEPTION 'contract is closed' USING ERRCODE='23514';
        END IF;
        IF NEW.kind='requested' THEN
            SELECT * INTO request FROM %1$I.contract_signature_requests WHERE request_id=NEW.request_id;
            IF request.requester_id IS DISTINCT FROM NEW.actor_id
                OR request.requester_email IS DISTINCT FROM NEW.actor_label
                OR ROW(request.created_at_seconds,request.created_at_nanoseconds)
                    IS DISTINCT FROM ROW(NEW.at_seconds,NEW.at_nanoseconds)
                OR EXISTS(SELECT 1 FROM %1$I.contract_events WHERE request_id=NEW.request_id) THEN
                RAISE EXCEPTION 'contract request event differs from its request' USING ERRCODE='23514';
            END IF;
        ELSIF staff IS NOT TRUE THEN
            RAISE EXCEPTION 'contract actor is not currently authorized' USING ERRCODE='42501';
        END IF;
    ELSE
        SELECT * INTO request FROM %1$I.contract_signature_requests WHERE request_id=NEW.request_id;
        IF previous.request_id IS DISTINCT FROM NEW.request_id
            OR NOT ((previous.kind='requested' AND NEW.kind IN ('client_signed','rejected','cancelled','expired'))
                OR (previous.kind='client_signed' AND NEW.kind IN ('completed','declined','cancelled','expired'))) THEN
            RAISE EXCEPTION 'contract event is out of turn' USING ERRCODE='23514';
        END IF;
        IF NEW.kind='expired' THEN
            IF NEW.at_seconds<request.expires_at_seconds THEN
                RAISE EXCEPTION 'contract request has not expired' USING ERRCODE='23514';
            END IF;
        ELSIF NEW.at_seconds>=request.expires_at_seconds THEN
            RAISE EXCEPTION 'contract request has expired' USING ERRCODE='23514';
        ELSIF (NEW.kind IN ('client_signed','rejected') AND NEW.actor_id IS DISTINCT FROM request.client_id)
            OR (NEW.kind='rejected' AND actor.role IS DISTINCT FROM 'client')
            OR (NEW.kind IN ('completed','declined') AND NEW.actor_id IS DISTINCT FROM request.firm_signer_id)
            OR (NEW.kind='declined' AND staff IS NOT TRUE)
            OR (NEW.kind='cancelled' AND actor.role IS DISTINCT FROM 'owner'
                AND (NEW.actor_id IS DISTINCT FROM request.requester_id OR staff IS NOT TRUE)) THEN
            RAISE EXCEPTION 'contract actor is not designated' USING ERRCODE='42501';
        END IF;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence
        AND a.actor=NEW.actor_label AND a.action='contract.' || NEW.kind
        AND a.resource=CASE NEW.kind
            WHEN 'created' THEN 'contract:' || NEW.contract_id::text || ':document:' || root.document_id::text
            WHEN 'abandoned' THEN 'contract:' || NEW.contract_id::text || ':revision:' || NEW.revision::text
            ELSE 'contract-request:' || NEW.request_id::text || ':revision:' || NEW.revision::text END
        AND a.timestamp_seconds=NEW.at_seconds AND a.timestamp_nanos=NEW.at_nanoseconds) THEN
        RAISE EXCEPTION 'contract event audit differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.contract_request_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE previous RECORD; prior RECORD; document RECORD; client RECORD; firm RECORD; requester RECORD;
    authority RECORD;
BEGIN
    SELECT * INTO previous FROM %1$I.contract_events WHERE contract_id=NEW.contract_id
        ORDER BY revision DESC LIMIT 1;
    IF previous.kind IN ('requested','client_signed') THEN
        SELECT * INTO prior FROM %1$I.contract_signature_requests WHERE request_id=previous.request_id;
        IF NEW.created_at_seconds<prior.expires_at_seconds THEN
            RAISE EXCEPTION 'contract request is open' USING ERRCODE='23514';
        END IF;
        RAISE EXCEPTION 'contract request must expire first' USING ERRCODE='23514';
    ELSIF previous.kind IS NULL OR previous.kind NOT IN ('created','rejected','declined','cancelled','expired') THEN
        RAISE EXCEPTION 'contract is closed' USING ERRCODE='23514';
    END IF;
    IF coalesce((SELECT administrative_status='closed' FROM %1$I.case_administration_revisions
        WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1),FALSE) THEN
        RAISE EXCEPTION 'contract case is closed' USING ERRCODE='42501';
    END IF;
    SELECT d.case_id,d.digest,d.evidence INTO document FROM %1$I.documents d
        WHERE d.id=NEW.document_id AND d.version=NEW.document_version FOR SHARE;
    IF document.case_id IS DISTINCT FROM NEW.case_id OR document.digest IS DISTINCT FROM NEW.content_digest
        OR document.evidence IS NOT NULL
        OR EXISTS(SELECT 1 FROM %1$I.documents WHERE id=NEW.document_id AND version>NEW.document_version) THEN
        RAISE EXCEPTION 'contract document version is not available' USING ERRCODE='23514';
    END IF;
    IF NEW.client_subject_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM %1$I.case_subjects
        WHERE id=NEW.client_subject_id AND case_id=NEW.case_id) THEN
        RAISE EXCEPTION 'contract subject belongs to another case' USING ERRCODE='23514';
    END IF;
    SELECT role,active INTO client FROM %1$I.users WHERE id=NEW.client_id FOR SHARE;
    IF client.active IS DISTINCT FROM TRUE OR client.role IS DISTINCT FROM 'client'
        OR NOT EXISTS(SELECT 1 FROM %1$I.member_invitation_completions c
            JOIN %1$I.member_invitations i ON i.id=c.invitation_id
            WHERE c.user_id=NEW.client_id AND i.role='client') THEN
        RAISE EXCEPTION 'contract client is not eligible' USING ERRCODE='42501';
    END IF;
    SELECT role,active INTO firm FROM %1$I.users WHERE id=NEW.firm_signer_id FOR SHARE;
    IF firm.active IS DISTINCT FROM TRUE OR NOT (firm.role='owner' OR (firm.role='litigator'
        AND EXISTS(SELECT 1 FROM %1$I.case_memberships
            WHERE case_id=NEW.case_id AND user_id=NEW.firm_signer_id))) THEN
        RAISE EXCEPTION 'contract firm signer is not eligible' USING ERRCODE='42501';
    END IF;
    SELECT email,role,active INTO requester FROM %1$I.users WHERE id=NEW.requester_id FOR SHARE;
    IF requester.active IS DISTINCT FROM TRUE OR requester.email IS DISTINCT FROM NEW.requester_email
        OR NOT (requester.role='owner' OR (requester.role='litigator'
            AND EXISTS(SELECT 1 FROM %1$I.case_memberships
                WHERE case_id=NEW.case_id AND user_id=NEW.requester_id))) THEN
        RAISE EXCEPTION 'contract requester is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO authority FROM %1$I.participant_credential_authority;
    IF authority.deployment_id IS DISTINCT FROM NEW.deployment_id OR EXISTS(
        SELECT 1 FROM %1$I.document_signing_bindings b
        WHERE b.binding_id IN (NEW.client_binding_id,NEW.firm_binding_id)
            AND (b.deployment_id<>NEW.deployment_id OR b.root_fingerprint<>authority.root_fingerprint
                OR ROW(b.registered_at_seconds,b.registered_at_nanoseconds)
                    >ROW(NEW.created_at_seconds,NEW.created_at_nanoseconds)
                OR EXISTS(SELECT 1 FROM %1$I.document_signing_withdrawals w
                    WHERE w.binding_id=b.binding_id))) THEN
        RAISE EXCEPTION 'contract party binding is inactive' USING ERRCODE='23514';
    END IF;
    IF NEW.statement IS DISTINCT FROM %1$I.contract_request_bytes(NEW.deployment_id,NEW.request_id,
        NEW.case_id,NEW.document_id,NEW.document_version,NEW.content_digest,NEW.client_id,
        NEW.firm_signer_id,NEW.requester_id,NEW.client_leaf,NEW.firm_leaf,NEW.created_at_seconds,
        NEW.expires_at_seconds) THEN
        RAISE EXCEPTION 'contract request bytes differ' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;
