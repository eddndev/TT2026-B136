CREATE TABLE IF NOT EXISTS owner_certificate_registrations (
    binding_id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id),
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
    CONSTRAINT owner_certificate_registrations_trust_fkey FOREIGN KEY(deployment_id,trust_revision)
        REFERENCES participant_credential_trust_revisions(deployment_id,revision),
    CONSTRAINT owner_certificate_registrations_audit_sequence_fkey FOREIGN KEY(audit_sequence)
        REFERENCES audit_events(sequence),
    CHECK(binding_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(owner_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(deployment_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(account_revision>=0 AND auth_generation>=0 AND auth_generation<=account_revision),
    CHECK(octet_length(actor_email) BETWEEN 1 AND 1280),
    CHECK(trust_revision BETWEEN 1 AND 4294967295),
    CHECK(octet_length(root_fingerprint)=32),
    CHECK(leaf_fingerprint=sha256(certificate_der)),
    CHECK(octet_length(statement)=150),
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

CREATE TABLE IF NOT EXISTS owner_certificate_withdrawals (
    binding_id UUID PRIMARY KEY REFERENCES owner_certificate_registrations(binding_id),
    account_revision BIGINT NOT NULL,
    auth_generation BIGINT NOT NULL,
    actor_email TEXT NOT NULL,
    statement BYTEA NOT NULL,
    statement_digest BYTEA NOT NULL,
    withdrawn_at_seconds BIGINT NOT NULL,
    withdrawn_at_nanoseconds INTEGER NOT NULL,
    audit_sequence BIGINT NOT NULL UNIQUE,
    CONSTRAINT owner_certificate_withdrawals_audit_sequence_fkey FOREIGN KEY(audit_sequence)
        REFERENCES audit_events(sequence),
    CHECK(account_revision>=0 AND auth_generation>=0 AND auth_generation<=account_revision),
    CHECK(octet_length(actor_email) BETWEEN 1 AND 1280),
    CHECK(octet_length(statement)=150),
    CHECK(statement_digest=sha256(statement)),
    CHECK(withdrawn_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(withdrawn_at_nanoseconds BETWEEN 0 AND 999999999),
    CHECK(audit_sequence>=0)
);

CREATE INDEX IF NOT EXISTS owner_certificate_owner_bindings
    ON owner_certificate_registrations(owner_id,binding_id);
CREATE INDEX IF NOT EXISTS owner_certificate_fingerprint_owners
    ON owner_certificate_registrations(leaf_fingerprint,owner_id);
REVOKE ALL ON owner_certificate_registrations,owner_certificate_withdrawals FROM PUBLIC;
