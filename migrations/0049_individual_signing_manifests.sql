CREATE TABLE IF NOT EXISTS individual_document_manifests (
    operation_id UUID PRIMARY KEY,
    document_id UUID NOT NULL,
    document_version BIGINT NOT NULL,
    manifest BYTEA NOT NULL,
    manifest_sha256 BYTEA NOT NULL,
    document_sha256 BYTEA NOT NULL,
    manifest_token BYTEA NOT NULL,
    tsa_crl_pem BYTEA NOT NULL,
    CONSTRAINT individual_manifest_receipt_fkey FOREIGN KEY(operation_id)
        REFERENCES individual_document_signatures(operation_id),
    CHECK(document_version BETWEEN 1 AND 4294967295),
    CHECK(octet_length(manifest) BETWEEN 1 AND 65536),
    CHECK(manifest_sha256=sha256(manifest)),
    CHECK(octet_length(document_sha256)=32),
    CHECK(octet_length(manifest_token) BETWEEN 1 AND 65536),
    CHECK(octet_length(tsa_crl_pem) BETWEEN 1 AND 1500000)
);
REVOKE ALL ON individual_document_manifests FROM PUBLIC;

CREATE TABLE IF NOT EXISTS individual_manifest_cutoff (
    singleton BOOLEAN PRIMARY KEY,
    first_audit_sequence BIGINT NOT NULL,
    CHECK(singleton),
    CHECK(first_audit_sequence>=1)
);
REVOKE ALL ON individual_manifest_cutoff FROM PUBLIC;
-- Receipts accepted before this migration keep no manifest; every later one needs one.
INSERT INTO individual_manifest_cutoff(singleton,first_audit_sequence)
    SELECT TRUE,coalesce(max(audit_sequence),0)+1 FROM individual_document_signatures
    ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION individual_manifest_preserve() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
BEGIN
    RAISE EXCEPTION 'individual signing manifests are immutable' USING ERRCODE='23514';
END;
$function$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.individual_manifest_register() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE receipt RECORD; cutoff BIGINT;
BEGIN
    SELECT first_audit_sequence INTO cutoff FROM %1$I.individual_manifest_cutoff WHERE singleton;
    SELECT r.document_id,r.document_version,r.content_digest,r.audit_sequence,d.digest INTO receipt
        FROM %1$I.individual_document_signatures r
        JOIN %1$I.documents d ON d.id=r.document_id AND d.version=r.document_version
        WHERE r.operation_id=NEW.operation_id;
    IF cutoff IS NULL OR receipt.audit_sequence IS NULL OR receipt.audit_sequence<cutoff
        OR receipt.document_id IS DISTINCT FROM NEW.document_id
        OR receipt.document_version IS DISTINCT FROM NEW.document_version
        OR receipt.content_digest IS DISTINCT FROM NEW.document_sha256
        OR receipt.digest IS DISTINCT FROM NEW.document_sha256 THEN
        RAISE EXCEPTION 'individual signing manifest differs from its receipt' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$function$;
$definition$,current_schema());
END; $install$;

DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.individual_manifest_pair() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog AS $function$
DECLARE cutoff BIGINT;
BEGIN
    SELECT first_audit_sequence INTO cutoff FROM %1$I.individual_manifest_cutoff WHERE singleton;
    IF cutoff IS NULL OR (NEW.audit_sequence>=cutoff AND NOT EXISTS(
        SELECT 1 FROM %1$I.individual_document_manifests m WHERE m.operation_id=NEW.operation_id
            AND m.document_id=NEW.document_id AND m.document_version=NEW.document_version
            AND m.document_sha256=NEW.content_digest)) THEN
        RAISE EXCEPTION 'individual signature requires its evidence manifest' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$function$;
$definition$,current_schema());
END; $install$;

DROP TRIGGER IF EXISTS individual_manifest_immutable ON individual_document_manifests;
CREATE TRIGGER individual_manifest_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON individual_document_manifests FOR EACH STATEMENT EXECUTE FUNCTION individual_manifest_preserve();
DROP TRIGGER IF EXISTS individual_manifest_cutoff_immutable ON individual_manifest_cutoff;
CREATE TRIGGER individual_manifest_cutoff_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON individual_manifest_cutoff FOR EACH STATEMENT EXECUTE FUNCTION individual_manifest_preserve();
DROP TRIGGER IF EXISTS individual_manifest_register ON individual_document_manifests;
CREATE TRIGGER individual_manifest_register BEFORE INSERT ON individual_document_manifests
    FOR EACH ROW EXECUTE FUNCTION individual_manifest_register();
DROP TRIGGER IF EXISTS individual_signature_manifest_pair ON individual_document_signatures;
CREATE CONSTRAINT TRIGGER individual_signature_manifest_pair AFTER INSERT ON individual_document_signatures
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION individual_manifest_pair();
REVOKE ALL ON FUNCTION individual_manifest_preserve(),individual_manifest_register(),
    individual_manifest_pair() FROM PUBLIC;
