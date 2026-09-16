CREATE TABLE IF NOT EXISTS participant_credential_authority (
    deployment_id UUID PRIMARY KEY,
    singleton BOOLEAN NOT NULL DEFAULT TRUE,
    root_der BYTEA NOT NULL,
    root_fingerprint BYTEA NOT NULL,
    initial_revision BIGINT NOT NULL DEFAULT 1,
    CONSTRAINT credential_authority_singleton CHECK(singleton),
    CONSTRAINT credential_authority_singleton_unique UNIQUE(singleton),
    CONSTRAINT credential_authority_initial CHECK(initial_revision=1),
    CONSTRAINT credential_authority_material CHECK(octet_length(root_der) BETWEEN 1 AND 16384),
    CONSTRAINT credential_authority_digest CHECK(root_fingerprint=pg_catalog.sha256(root_der))
);

CREATE TABLE IF NOT EXISTS participant_credential_trust_revisions (
    deployment_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    crl_der BYTEA NOT NULL,
    crl_digest BYTEA NOT NULL,
    crl_number NUMERIC(20,0) NOT NULL,
    crl_this_update BIGINT NOT NULL,
    crl_next_update BIGINT NOT NULL,
    valid_from BIGINT NOT NULL,
    valid_until BIGINT NOT NULL,
    published_at_seconds BIGINT NOT NULL,
    published_at_nanoseconds INTEGER NOT NULL,
    published_by TEXT NOT NULL,
    PRIMARY KEY(deployment_id,revision),
    CONSTRAINT credential_trust_root_fk FOREIGN KEY(deployment_id)
        REFERENCES participant_credential_authority(deployment_id),
    CONSTRAINT credential_trust_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    CONSTRAINT credential_trust_material CHECK(octet_length(crl_der) BETWEEN 1 AND 1048576),
    CONSTRAINT credential_trust_digest CHECK(crl_digest=pg_catalog.sha256(crl_der)),
    CONSTRAINT credential_trust_number_range CHECK(crl_number BETWEEN 0 AND 18446744073709551615),
    CONSTRAINT credential_trust_window CHECK(crl_this_update>=0
        AND crl_next_update>crl_this_update AND crl_next_update<=253402300799
        AND valid_from>=crl_this_update AND valid_until<=crl_next_update AND valid_from<=valid_until),
    CONSTRAINT credential_trust_published_time CHECK(published_at_seconds BETWEEN valid_from AND valid_until
        AND published_at_nanoseconds BETWEEN 0 AND 999999999),
    CONSTRAINT credential_trust_published_by CHECK(octet_length(published_by) BETWEEN 1 AND 1024)
);

DO $constraint$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint
        WHERE conrelid='participant_credential_authority'::regclass
        AND conname='credential_authority_initial_fk') THEN
        ALTER TABLE participant_credential_authority ADD CONSTRAINT credential_authority_initial_fk
            FOREIGN KEY(deployment_id,initial_revision)
            REFERENCES participant_credential_trust_revisions(deployment_id,revision)
            DEFERRABLE INITIALLY DEFERRED;
    END IF;
END
$constraint$;

CREATE OR REPLACE FUNCTION preserve_credential_trust_history() RETURNS TRIGGER
LANGUAGE plpgsql AS $function$
BEGIN
    RAISE EXCEPTION 'credential trust history is immutable' USING ERRCODE='23514';
END
$function$;

DO $definition$
BEGIN
    EXECUTE format($function$
CREATE OR REPLACE FUNCTION %1$I.enforce_credential_trust_sequence() RETURNS TRIGGER
LANGUAGE plpgsql AS $body$
DECLARE
    previous_revision BIGINT;
    previous_number NUMERIC;
    previous_update BIGINT;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'credential trust publication requires read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    SELECT revision,crl_number,crl_this_update
        INTO previous_revision,previous_number,previous_update
        FROM %1$I.participant_credential_trust_revisions
        WHERE deployment_id=NEW.deployment_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision<>COALESCE(previous_revision,0)+1 THEN
        RAISE EXCEPTION 'credential trust revisions must be contiguous' USING ERRCODE='23514';
    END IF;
    IF previous_revision IS NOT NULL AND
        (NEW.crl_number<=previous_number OR NEW.crl_this_update<previous_update) THEN
        RAISE EXCEPTION 'credential revocation lists must advance without moving backwards' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END
$body$;
$function$,current_schema());
END
$definition$;

DROP TRIGGER IF EXISTS credential_authority_immutable ON participant_credential_authority;
CREATE TRIGGER credential_authority_immutable BEFORE UPDATE OR DELETE ON participant_credential_authority
    FOR EACH ROW EXECUTE FUNCTION preserve_credential_trust_history();
DROP TRIGGER IF EXISTS credential_authority_no_truncate ON participant_credential_authority;
CREATE TRIGGER credential_authority_no_truncate BEFORE TRUNCATE ON participant_credential_authority
    FOR EACH STATEMENT EXECUTE FUNCTION preserve_credential_trust_history();
DROP TRIGGER IF EXISTS credential_trust_immutable ON participant_credential_trust_revisions;
CREATE TRIGGER credential_trust_immutable BEFORE UPDATE OR DELETE ON participant_credential_trust_revisions
    FOR EACH ROW EXECUTE FUNCTION preserve_credential_trust_history();
DROP TRIGGER IF EXISTS credential_trust_no_truncate ON participant_credential_trust_revisions;
CREATE TRIGGER credential_trust_no_truncate BEFORE TRUNCATE ON participant_credential_trust_revisions
    FOR EACH STATEMENT EXECUTE FUNCTION preserve_credential_trust_history();
DROP TRIGGER IF EXISTS credential_trust_sequence ON participant_credential_trust_revisions;
CREATE TRIGGER credential_trust_sequence BEFORE INSERT ON participant_credential_trust_revisions
    FOR EACH ROW EXECUTE FUNCTION enforce_credential_trust_sequence();

REVOKE ALL ON participant_credential_authority,participant_credential_trust_revisions FROM PUBLIC;
REVOKE ALL ON FUNCTION preserve_credential_trust_history(),enforce_credential_trust_sequence() FROM PUBLIC;
