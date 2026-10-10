CREATE TABLE IF NOT EXISTS individual_document_signatures (
    operation_id UUID PRIMARY KEY,
    case_id UUID NOT NULL,
    document_id UUID NOT NULL,
    document_version BIGINT NOT NULL,
    signer_id UUID NOT NULL,
    binding_id UUID NOT NULL,
    deployment_id UUID NOT NULL,
    trust_revision BIGINT NOT NULL,
    root_fingerprint BYTEA NOT NULL,
    leaf_fingerprint BYTEA NOT NULL,
    account_revision BIGINT NOT NULL,
    auth_generation BIGINT NOT NULL,
    actor_email TEXT NOT NULL,
    consent BYTEA NOT NULL,
    consent_digest BYTEA NOT NULL,
    content_digest BYTEA NOT NULL,
    pdf_signature BYTEA NOT NULL,
    consent_signature BYTEA NOT NULL,
    checked_at BIGINT NOT NULL,
    valid_from BIGINT NOT NULL,
    valid_until BIGINT NOT NULL,
    accepted_at_seconds BIGINT NOT NULL,
    accepted_at_nanoseconds INTEGER NOT NULL,
    audit_sequence BIGINT NOT NULL UNIQUE,
    UNIQUE(document_id,document_version),
    CONSTRAINT individual_signature_binding_fkey FOREIGN KEY(binding_id,signer_id,leaf_fingerprint)
        REFERENCES document_signing_bindings(binding_id,signer_id,leaf_fingerprint),
    CONSTRAINT individual_signature_document_fkey FOREIGN KEY(document_id,document_version)
        REFERENCES documents(id,version),
    CONSTRAINT individual_signature_case_fkey FOREIGN KEY(document_id,case_id)
        REFERENCES document_series(id,case_id),
    CONSTRAINT individual_signature_trust_fkey FOREIGN KEY(deployment_id,trust_revision)
        REFERENCES participant_credential_trust_revisions(deployment_id,revision),
    CONSTRAINT individual_signature_audit_fkey FOREIGN KEY(audit_sequence) REFERENCES audit_events(sequence),
    CHECK(operation_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(case_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(document_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(signer_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(binding_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(deployment_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(document_version BETWEEN 1 AND 4294967295),
    CHECK(trust_revision BETWEEN 1 AND 4294967295),
    CHECK(account_revision>=0 AND auth_generation>=0 AND auth_generation<=account_revision),
    CHECK(octet_length(actor_email) BETWEEN 1 AND 1280),
    CHECK(octet_length(root_fingerprint)=32),
    CHECK(octet_length(leaf_fingerprint)=32),
    CHECK(octet_length(consent)=258),
    CHECK(consent_digest=sha256(consent)),
    CHECK(octet_length(content_digest)=32),
    CHECK(octet_length(pdf_signature)=384),
    CHECK(octet_length(consent_signature)=384),
    CHECK(valid_from<=checked_at AND checked_at<=valid_until),
    CHECK(accepted_at_seconds>=checked_at AND accepted_at_seconds<=valid_until),
    CHECK(accepted_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(accepted_at_nanoseconds BETWEEN 0 AND 999999999),
    CHECK(audit_sequence>=0)
);
REVOKE ALL ON individual_document_signatures FROM PUBLIC;

CREATE OR REPLACE FUNCTION individual_signature_lock() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'individual signing requires read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END;
$function$;

CREATE OR REPLACE FUNCTION individual_signature_preserve() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'individual signature receipts are immutable' USING ERRCODE='23514';
END;
$function$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.individual_signature_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE actor RECORD; trust RECORD; binding RECORD; canonical BYTEA;
BEGIN
    SELECT email,role,active,revision,auth_generation INTO actor
        FROM %1$I.users WHERE id=NEW.signer_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.role IS DISTINCT FROM 'litigator'
        OR actor.email IS DISTINCT FROM NEW.actor_email
        OR actor.revision IS DISTINCT FROM NEW.account_revision
        OR actor.auth_generation IS DISTINCT FROM NEW.auth_generation
        OR NOT EXISTS(SELECT 1 FROM %1$I.case_memberships
            WHERE case_id=NEW.case_id AND user_id=NEW.signer_id)
        OR coalesce((SELECT administrative_status='closed' FROM %1$I.case_administration_revisions
            WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1),FALSE) THEN
        RAISE EXCEPTION 'individual signer is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO binding FROM %1$I.document_signing_bindings WHERE binding_id=NEW.binding_id;
    IF binding.signer_id IS DISTINCT FROM NEW.signer_id
        OR binding.leaf_fingerprint IS DISTINCT FROM NEW.leaf_fingerprint
        OR binding.deployment_id IS DISTINCT FROM NEW.deployment_id
        OR binding.root_fingerprint IS DISTINCT FROM NEW.root_fingerprint
        OR binding.account_revision>NEW.account_revision OR binding.auth_generation>NEW.auth_generation
        OR binding.trust_revision>NEW.trust_revision OR binding.audit_sequence>=NEW.audit_sequence
        OR ROW(binding.registered_at_seconds,binding.registered_at_nanoseconds)
            >ROW(NEW.accepted_at_seconds,NEW.accepted_at_nanoseconds)
        OR EXISTS(SELECT 1 FROM %1$I.document_signing_withdrawals WHERE binding_id=NEW.binding_id) THEN
        RAISE EXCEPTION 'individual signing binding is inactive or inconsistent' USING ERRCODE='23514';
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
        RAISE EXCEPTION 'individual signing current trust differs' USING ERRCODE='23514';
    END IF;
    canonical := convert_to('DOCSIGN1','UTF8') || decode('0101','hex')
        || uuid_send(NEW.deployment_id) || NEW.root_fingerprint
        || substring(int8send(NEW.trust_revision) FROM 5 FOR 4)
        || uuid_send(NEW.signer_id) || int8send(NEW.account_revision) || int8send(NEW.auth_generation)
        || uuid_send(NEW.binding_id) || NEW.leaf_fingerprint || uuid_send(NEW.operation_id)
        || uuid_send(NEW.case_id) || uuid_send(NEW.document_id)
        || substring(int8send(NEW.document_version) FROM 5 FOR 4)
        || NEW.content_digest || sha256(NEW.pdf_signature);
    IF NEW.consent IS DISTINCT FROM canonical THEN
        RAISE EXCEPTION 'individual consent bytes differ' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM %1$I.audit_events a WHERE a.sequence=NEW.audit_sequence
        AND a.actor=NEW.actor_email AND a.action='document.individually_signed'
        AND a.resource='case:' || NEW.case_id::text || ':document:' || NEW.document_id::text
            || ':version:' || NEW.document_version::text || ':signer:' || NEW.signer_id::text
            || ':operation:' || NEW.operation_id::text || ':consent:' || encode(NEW.consent_digest,'hex')
        AND a.timestamp_seconds=NEW.accepted_at_seconds AND a.timestamp_nanos=NEW.accepted_at_nanoseconds) THEN
        RAISE EXCEPTION 'individual signing audit differs' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.individual_signature_pair() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE target_id UUID; target_version BIGINT; document RECORD; receipt RECORD;
    binding RECORD; trust RECORD; original_trust RECORD; expected JSONB;
    material RECORD; encoded TEXT; pem TEXT;
BEGIN
    IF TG_TABLE_NAME='documents' THEN
        target_id:=NEW.id; target_version:=NEW.version;
    ELSE
        target_id:=NEW.document_id; target_version:=NEW.document_version;
    END IF;
    SELECT d.* INTO document FROM %1$I.documents d
        WHERE d.id=target_id AND d.version=target_version;
    SELECT r.* INTO receipt FROM %1$I.individual_document_signatures r
        WHERE r.document_id=target_id AND r.document_version=target_version;
    IF receipt.operation_id IS NULL THEN
        IF document.evidence->'individual' IS NOT NULL AND document.evidence->'individual'<>'null'::jsonb THEN
            RAISE EXCEPTION 'individual document evidence requires a receipt' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
    SELECT * INTO binding FROM %1$I.document_signing_bindings WHERE binding_id=receipt.binding_id;
    SELECT t.*,a.root_der INTO trust FROM %1$I.participant_credential_trust_revisions t
        JOIN %1$I.participant_credential_authority a USING(deployment_id)
        WHERE t.deployment_id=receipt.deployment_id AND t.revision=receipt.trust_revision;
    SELECT * INTO original_trust FROM %1$I.participant_credential_trust_revisions
        WHERE deployment_id=binding.deployment_id AND revision=binding.trust_revision;
    expected:=jsonb_build_object('format_version',1,
        'binding_statement',to_jsonb(ARRAY(SELECT get_byte(binding.statement,n) FROM generate_series(0,141) n)),
        'binding_signature',to_jsonb(ARRAY(SELECT get_byte(binding.signature,n) FROM generate_series(0,383) n)),
        'binding_root_der',to_jsonb(ARRAY(SELECT get_byte(trust.root_der,n)
            FROM generate_series(0,octet_length(trust.root_der)-1) n)),
        'binding_crl_der',to_jsonb(ARRAY(SELECT get_byte(original_trust.crl_der,n)
            FROM generate_series(0,octet_length(original_trust.crl_der)-1) n)),
        'binding_registered_at',binding.registered_at_seconds,
        'consent_statement',to_jsonb(ARRAY(SELECT get_byte(receipt.consent,n) FROM generate_series(0,257) n)),
        'consent_signature',to_jsonb(ARRAY(SELECT get_byte(receipt.consent_signature,n) FROM generate_series(0,383) n)),
        'accepted_at',receipt.accepted_at_seconds);
    IF document.case_id IS DISTINCT FROM receipt.case_id OR document.digest IS DISTINCT FROM receipt.content_digest
        OR document.evidence->'individual' IS DISTINCT FROM expected
        OR decode(document.evidence->>'signature_base64','base64') IS DISTINCT FROM receipt.pdf_signature THEN
        RAISE EXCEPTION 'individual signature and document evidence differ' USING ERRCODE='23514';
    END IF;
    FOR material IN SELECT * FROM (VALUES
        ('signer_certificate_base64','CERTIFICATE',binding.certificate_der),
        ('issuer_certificate_base64','CERTIFICATE',trust.root_der),
        ('crl_base64','X509 CRL',trust.crl_der)) AS m(field,label,der) LOOP
        encoded:=replace(encode(material.der,'base64'),E'\n','');
        SELECT '-----BEGIN ' || material.label || E'-----\n'
            || string_agg(substring(encoded FROM n FOR 64) || E'\n','' ORDER BY n)
            || '-----END ' || material.label || E'-----\n' INTO pem
            FROM generate_series(1,length(encoded),64) n;
        IF decode(document.evidence->>material.field,'base64') IS DISTINCT FROM convert_to(pem,'UTF8') THEN
            RAISE EXCEPTION 'individual public material differs' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END;
$function$;
$definition$,current_schema());
END; $install$;

DROP TRIGGER IF EXISTS individual_signature_lock ON individual_document_signatures;
CREATE TRIGGER individual_signature_lock BEFORE INSERT ON individual_document_signatures
    FOR EACH STATEMENT EXECUTE FUNCTION individual_signature_lock();
DROP TRIGGER IF EXISTS individual_signature_immutable ON individual_document_signatures;
CREATE TRIGGER individual_signature_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON individual_document_signatures
    FOR EACH STATEMENT EXECUTE FUNCTION individual_signature_preserve();
DROP TRIGGER IF EXISTS individual_signature_register ON individual_document_signatures;
CREATE TRIGGER individual_signature_register BEFORE INSERT ON individual_document_signatures
    FOR EACH ROW EXECUTE FUNCTION individual_signature_register();
DROP TRIGGER IF EXISTS individual_signature_receipt_pair ON individual_document_signatures;
CREATE CONSTRAINT TRIGGER individual_signature_receipt_pair AFTER INSERT ON individual_document_signatures
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION individual_signature_pair();
DROP TRIGGER IF EXISTS individual_signature_document_pair ON documents;
CREATE CONSTRAINT TRIGGER individual_signature_document_pair AFTER INSERT OR UPDATE ON documents
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION individual_signature_pair();
REVOKE ALL ON FUNCTION individual_signature_lock(),individual_signature_preserve(),
    individual_signature_register(),individual_signature_pair() FROM PUBLIC;
