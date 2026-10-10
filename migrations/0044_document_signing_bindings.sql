CREATE TABLE IF NOT EXISTS document_signing_bindings (
    binding_id UUID PRIMARY KEY,
    signer_id UUID NOT NULL REFERENCES users(id),
    CONSTRAINT document_signing_binding_identity UNIQUE(binding_id,signer_id,leaf_fingerprint),
    account_revision BIGINT NOT NULL,
    auth_generation BIGINT NOT NULL,
    actor_email TEXT NOT NULL,
    deployment_id UUID NOT NULL,
    trust_revision BIGINT NOT NULL,
    root_fingerprint BYTEA NOT NULL,
    leaf_fingerprint BYTEA NOT NULL,
    statement BYTEA NOT NULL,
    statement_digest BYTEA NOT NULL,
    certificate_der BYTEA NOT NULL,
    signature BYTEA NOT NULL,
    certificate_subject TEXT NOT NULL,
    certificate_issuer TEXT NOT NULL,
    certificate_serial TEXT NOT NULL,
    certificate_not_before BIGINT NOT NULL,
    certificate_not_after BIGINT NOT NULL,
    checked_at BIGINT NOT NULL,
    valid_from BIGINT NOT NULL,
    valid_until BIGINT NOT NULL,
    registered_at_seconds BIGINT NOT NULL,
    registered_at_nanoseconds INTEGER NOT NULL,
    audit_sequence BIGINT NOT NULL UNIQUE,
    CONSTRAINT document_signing_bindings_trust_fkey FOREIGN KEY(deployment_id,trust_revision)
        REFERENCES participant_credential_trust_revisions(deployment_id,revision),
    CONSTRAINT document_signing_bindings_audit_sequence_fkey FOREIGN KEY(audit_sequence)
        REFERENCES audit_events(sequence),
    CHECK(binding_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(signer_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(deployment_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(account_revision>=0 AND auth_generation>=0 AND auth_generation<=account_revision),
    CHECK(octet_length(actor_email) BETWEEN 1 AND 1280),
    CHECK(trust_revision BETWEEN 1 AND 4294967295),
    CHECK(octet_length(root_fingerprint)=32),
    CHECK(leaf_fingerprint=sha256(certificate_der)),
    CHECK(octet_length(statement)=142),
    CHECK(statement_digest=sha256(statement)),
    CHECK(octet_length(certificate_der) BETWEEN 1 AND 16384),
    CHECK(octet_length(signature)=384),
    CHECK(octet_length(certificate_subject) BETWEEN 1 AND 65536),
    CHECK(octet_length(certificate_issuer) BETWEEN 1 AND 65536),
    CHECK(certificate_serial COLLATE "C" ~ '^[0-9A-F]{1,40}$'),
    CHECK(certificate_not_before<=certificate_not_after),
    CHECK(valid_from<=checked_at AND checked_at<=valid_until),
    CHECK(registered_at_seconds>=checked_at AND registered_at_seconds<=valid_until),
    CHECK(registered_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(registered_at_nanoseconds BETWEEN 0 AND 999999999),
    CHECK(audit_sequence>=0)
);

CREATE TABLE IF NOT EXISTS document_signing_withdrawals (
    binding_id UUID PRIMARY KEY REFERENCES document_signing_bindings(binding_id),
    account_revision BIGINT NOT NULL,
    auth_generation BIGINT NOT NULL,
    actor_email TEXT NOT NULL,
    withdrawn_at_seconds BIGINT NOT NULL,
    withdrawn_at_nanoseconds INTEGER NOT NULL,
    audit_sequence BIGINT NOT NULL UNIQUE,
    CONSTRAINT document_signing_withdrawals_audit_sequence_fkey FOREIGN KEY(audit_sequence)
        REFERENCES audit_events(sequence),
    CHECK(account_revision>=0 AND auth_generation>=0 AND auth_generation<=account_revision),
    CHECK(octet_length(actor_email) BETWEEN 1 AND 1280),
    CHECK(withdrawn_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(withdrawn_at_nanoseconds BETWEEN 0 AND 999999999),
    CHECK(audit_sequence>=0)
);

CREATE INDEX IF NOT EXISTS document_signing_signer_bindings
    ON document_signing_bindings(signer_id,binding_id);
CREATE INDEX IF NOT EXISTS document_signing_fingerprint_signers
    ON document_signing_bindings(leaf_fingerprint,signer_id);
REVOKE ALL ON document_signing_bindings,document_signing_withdrawals FROM PUBLIC;

CREATE OR REPLACE FUNCTION document_signing_lock() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'document signing writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    RETURN NULL;
END;
$function$;

CREATE OR REPLACE FUNCTION document_signing_preserve() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'document signing history is immutable' USING ERRCODE='23514';
END;
$function$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.document_signing_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE actor RECORD; trust RECORD; canonical BYTEA;
BEGIN
    SELECT email,role,active,revision,auth_generation INTO actor
        FROM %1$I.users WHERE id=NEW.signer_id FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.role IS DISTINCT FROM 'litigator'
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
        OR actor.role IS DISTINCT FROM 'litigator' OR actor.email IS DISTINCT FROM NEW.actor_email
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

DO $install$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['document_signing_bindings','document_signing_withdrawals'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='document_signing_lock') THEN
            EXECUTE format('CREATE TRIGGER document_signing_lock BEFORE INSERT ON %I FOR EACH STATEMENT EXECUTE FUNCTION document_signing_lock()',tab);
        END IF;
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='document_signing_immutable') THEN
            EXECUTE format('CREATE TRIGGER document_signing_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION document_signing_preserve()',tab);
        END IF;
    END LOOP;
END; $install$;
DROP TRIGGER IF EXISTS document_signing_register ON document_signing_bindings;
CREATE TRIGGER document_signing_register BEFORE INSERT ON document_signing_bindings
    FOR EACH ROW EXECUTE FUNCTION document_signing_register();
DROP TRIGGER IF EXISTS document_signing_withdraw ON document_signing_withdrawals;
CREATE TRIGGER document_signing_withdraw BEFORE INSERT ON document_signing_withdrawals
    FOR EACH ROW EXECUTE FUNCTION document_signing_withdraw();
REVOKE ALL ON FUNCTION document_signing_lock(),document_signing_preserve(),
    document_signing_register(),document_signing_withdraw() FROM PUBLIC;
