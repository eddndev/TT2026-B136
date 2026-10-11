CREATE TABLE IF NOT EXISTS member_invitations (
    id UUID PRIMARY KEY CHECK(id<>'00000000-0000-0000-0000-000000000000'::uuid),
    email TEXT NOT NULL UNIQUE CHECK(length(email) BETWEEN 3 AND 254 AND email=lower(btrim(email))),
    role TEXT NOT NULL CHECK(role IN ('litigator','paralegal')),
    revision BIGINT NOT NULL CHECK(revision>0),
    operation_id UUID NOT NULL,
    issued_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL CHECK(expires_at>issued_at),
    accepted_at BIGINT,
    cancelled_at BIGINT,
    token_digest BYTEA NOT NULL UNIQUE CHECK(octet_length(token_digest)=32),
    protected_token BYTEA,
    pending_operation UUID,
    pending_digest BYTEA,
    pending_material BYTEA,
    delivery_state TEXT NOT NULL CHECK(delivery_state IN ('pending','sent','uncertain','rejected')),
    delivery_generation BIGINT NOT NULL DEFAULT 0 CHECK(delivery_generation>=0),
    lease_id UUID,
    lease_until BIGINT,
    provider_deadline BIGINT,
    retry_at BIGINT,
    audit_sequence BIGINT NOT NULL REFERENCES audit_events(sequence),
    CHECK((pending_operation IS NULL AND pending_digest IS NULL AND pending_material IS NULL)
        OR (pending_operation IS NOT NULL AND pending_digest IS NOT NULL
            AND pending_material IS NOT NULL AND octet_length(pending_digest)=32
            AND octet_length(pending_material) BETWEEN 29 AND 65564)),
    CHECK((accepted_at IS NULL AND cancelled_at IS NULL AND protected_token IS NOT NULL
            AND octet_length(protected_token) BETWEEN 29 AND 65564)
        OR ((accepted_at IS NOT NULL OR cancelled_at IS NOT NULL)
            AND protected_token IS NULL AND pending_material IS NULL)),
    CHECK(cancelled_at IS NULL OR (accepted_at IS NULL AND cancelled_at>=issued_at
        AND protected_token IS NULL AND pending_material IS NULL
        AND lease_id IS NULL AND retry_at IS NULL)),
    CHECK((lease_id IS NULL)=(lease_until IS NULL))
);
CREATE TABLE IF NOT EXISTS member_invitation_operations (
    operation_id UUID PRIMARY KEY,
    actor_id UUID NOT NULL REFERENCES users(id),
    invitation_id UUID NOT NULL REFERENCES member_invitations(id),
    command JSONB NOT NULL,
    response JSONB NOT NULL,
    audit_sequence BIGINT NOT NULL UNIQUE REFERENCES audit_events(sequence)
);
CREATE TABLE IF NOT EXISTS member_invitation_completions (
    invitation_id UUID PRIMARY KEY REFERENCES member_invitations(id),
    operation_id UUID NOT NULL UNIQUE,
    token_digest BYTEA NOT NULL UNIQUE CHECK(octet_length(token_digest)=32),
    request_digest BYTEA NOT NULL CHECK(octet_length(request_digest)=32),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id),
    receipt JSONB NOT NULL,
    audit_sequence BIGINT NOT NULL UNIQUE REFERENCES audit_events(sequence)
);
CREATE INDEX IF NOT EXISTS member_invitation_deliveries
    ON member_invitations(delivery_state,retry_at,id)
    WHERE accepted_at IS NULL AND cancelled_at IS NULL;
REVOKE ALL ON member_invitations,member_invitation_operations,member_invitation_completions FROM PUBLIC;
