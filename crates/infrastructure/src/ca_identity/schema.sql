-- Run only with the trusted provisioning owner in the existing office database.
CREATE SCHEMA qadra_ca;
REVOKE ALL ON SCHEMA qadra_ca FROM PUBLIC;

CREATE TABLE qadra_ca.users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role = 'client'),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    protected_totp_secret BYTEA NOT NULL,
    recovery_codes JSONB NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    auth_generation BIGINT NOT NULL DEFAULT 0 CHECK (auth_generation >= 0)
);

CREATE TABLE qadra_ca.audit_events (
    sequence BIGINT PRIMARY KEY CHECK (sequence >= 0),
    timestamp TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    chain BYTEA NOT NULL CHECK (octet_length(chain) = 32),
    context_json TEXT CHECK (octet_length(context_json) <= 1024)
);

REVOKE ALL ON qadra_ca.users, qadra_ca.audit_events FROM PUBLIC;
