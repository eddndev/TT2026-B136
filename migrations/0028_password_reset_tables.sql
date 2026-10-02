CREATE TABLE IF NOT EXISTS password_reset_capabilities (
    id UUID CONSTRAINT password_reset_capabilities_pkey PRIMARY KEY,
    digest BYTEA NOT NULL CONSTRAINT password_reset_capabilities_digest_key UNIQUE,
    user_id UUID NOT NULL,
    email TEXT NOT NULL,
    auth_generation BIGINT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    cancelled_at TIMESTAMPTZ,
    consumed_at TIMESTAMPTZ,
    consumed_revision BIGINT,
    consumed_generation BIGINT,
    audit_sequence BIGINT CONSTRAINT password_reset_capabilities_audit_sequence_key UNIQUE,
    CONSTRAINT password_reset_id_check CHECK (id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CONSTRAINT password_reset_digest_check CHECK (octet_length(digest)=32),
    CONSTRAINT password_reset_generation_check CHECK (auth_generation>=0),
    CONSTRAINT password_reset_time_check CHECK (isfinite(issued_at) AND isfinite(expires_at)
        AND extract(epoch FROM issued_at)>='-62135596800'::numeric
        AND extract(epoch FROM expires_at)<'253402300800'::numeric AND expires_at>issued_at),
    CONSTRAINT password_reset_cancel_check CHECK (cancelled_at IS NULL OR
        (isfinite(cancelled_at) AND cancelled_at>=issued_at
        AND extract(epoch FROM cancelled_at)<'253402300800'::numeric)),
    CONSTRAINT password_reset_consumption_check CHECK (
        (consumed_at IS NULL AND consumed_revision IS NULL
            AND consumed_generation IS NULL AND audit_sequence IS NULL)
        OR (consumed_at IS NOT NULL AND consumed_at>=issued_at
            AND consumed_at<expires_at AND cancelled_at IS NULL
            AND consumed_revision IS NOT NULL AND consumed_revision>0
            AND consumed_generation IS NOT NULL AND consumed_generation>0
            AND consumed_generation::numeric=auth_generation::numeric+'1'::numeric
            AND consumed_generation<=consumed_revision
            AND audit_sequence IS NOT NULL AND audit_sequence>=0)),
    CONSTRAINT password_reset_capabilities_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT password_reset_capabilities_audit_sequence_fkey FOREIGN KEY (audit_sequence)
        REFERENCES audit_events(sequence) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS password_reset_account_generation
    ON password_reset_capabilities(user_id,auth_generation);
REVOKE ALL ON password_reset_capabilities FROM PUBLIC;
