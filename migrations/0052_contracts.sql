-- Bilateral contract storage: docs/adr/0086-bilateral-contract-signing.md.
-- Five append-only tables hold the contract, its signature requests, their
-- events, the party signatures and the completion; only the notification
-- inbox has mutable delivery columns. No table holds a state column.
CREATE TABLE IF NOT EXISTS contracts (
    id UUID NOT NULL,
    case_id UUID NOT NULL,
    document_id UUID NOT NULL,
    title TEXT NOT NULL,
    created_by UUID NOT NULL,
    created_by_email TEXT NOT NULL,
    created_at_seconds BIGINT NOT NULL,
    created_at_nanoseconds INTEGER NOT NULL,
    CONSTRAINT contracts_pkey PRIMARY KEY(id),
    CONSTRAINT contracts_document_key UNIQUE(document_id),
    CONSTRAINT contracts_identity_key UNIQUE(id,case_id,document_id),
    CONSTRAINT contracts_series_fkey FOREIGN KEY(document_id,case_id)
        REFERENCES document_series(id,case_id),
    CONSTRAINT contracts_creator_fkey FOREIGN KEY(created_by) REFERENCES users(id),
    CHECK(id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(case_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(document_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(char_length(title) BETWEEN 1 AND 160),
    CHECK(octet_length(created_by_email) BETWEEN 1 AND 1280),
    CHECK(created_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(created_at_nanoseconds BETWEEN 0 AND 999999999)
);

CREATE TABLE IF NOT EXISTS contract_signature_requests (
    request_id UUID NOT NULL,
    contract_id UUID NOT NULL,
    case_id UUID NOT NULL,
    document_id UUID NOT NULL,
    document_version BIGINT NOT NULL,
    content_digest BYTEA NOT NULL,
    client_id UUID NOT NULL,
    firm_signer_id UUID NOT NULL,
    requester_id UUID NOT NULL,
    requester_email TEXT NOT NULL,
    client_binding_id UUID NOT NULL,
    client_leaf BYTEA NOT NULL,
    firm_binding_id UUID NOT NULL,
    firm_leaf BYTEA NOT NULL,
    client_subject_id UUID,
    client_subject_revision BIGINT,
    ceremony_reference TEXT NOT NULL,
    deployment_id UUID NOT NULL,
    statement BYTEA NOT NULL,
    statement_digest BYTEA NOT NULL,
    created_at_seconds BIGINT NOT NULL,
    created_at_nanoseconds INTEGER NOT NULL,
    expires_at_seconds BIGINT NOT NULL,
    CONSTRAINT contract_requests_pkey PRIMARY KEY(request_id),
    CONSTRAINT contract_requests_contract_key UNIQUE(request_id,contract_id),
    CONSTRAINT contract_requests_contract_fkey FOREIGN KEY(contract_id,case_id,document_id)
        REFERENCES contracts(id,case_id,document_id),
    CONSTRAINT contract_requests_document_fkey FOREIGN KEY(document_id,document_version)
        REFERENCES documents(id,version),
    CONSTRAINT contract_requests_client_binding_fkey FOREIGN KEY(client_binding_id,client_id,client_leaf)
        REFERENCES document_signing_bindings(binding_id,signer_id,leaf_fingerprint),
    CONSTRAINT contract_requests_firm_binding_fkey FOREIGN KEY(firm_binding_id,firm_signer_id,firm_leaf)
        REFERENCES document_signing_bindings(binding_id,signer_id,leaf_fingerprint),
    CONSTRAINT contract_requests_requester_fkey FOREIGN KEY(requester_id) REFERENCES users(id),
    CONSTRAINT contract_requests_subject_fkey FOREIGN KEY(client_subject_id,client_subject_revision)
        REFERENCES case_subject_revisions(subject_id,revision),
    CHECK(request_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(deployment_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(document_version BETWEEN 1 AND 4294967295),
    CHECK(octet_length(content_digest)=32),
    CHECK(client_id<>firm_signer_id AND client_id<>requester_id),
    CHECK(client_leaf<>firm_leaf),
    CHECK(octet_length(requester_email) BETWEEN 1 AND 1280),
    CHECK((client_subject_id IS NULL)=(client_subject_revision IS NULL)),
    CHECK(char_length(ceremony_reference) BETWEEN 1 AND 80),
    CHECK(octet_length(statement)=240),
    CHECK(statement_digest=sha256(statement)),
    CHECK(created_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(created_at_nanoseconds BETWEEN 0 AND 999999999),
    CHECK(expires_at_seconds-created_at_seconds BETWEEN 86400 AND 2592000),
    CHECK(expires_at_seconds<=253402300799)
);

CREATE TABLE IF NOT EXISTS contract_events (
    contract_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    kind TEXT NOT NULL,
    request_id UUID,
    actor_id UUID,
    actor_label TEXT NOT NULL,
    reason TEXT,
    at_seconds BIGINT NOT NULL,
    at_nanoseconds INTEGER NOT NULL,
    operation_id UUID NOT NULL,
    audit_sequence BIGINT NOT NULL,
    CONSTRAINT contract_events_pkey PRIMARY KEY(contract_id,revision),
    CONSTRAINT contract_events_operation_key UNIQUE(operation_id),
    CONSTRAINT contract_events_audit_sequence_key UNIQUE(audit_sequence),
    CONSTRAINT contract_events_contract_fkey FOREIGN KEY(contract_id) REFERENCES contracts(id),
    CONSTRAINT contract_events_request_fkey FOREIGN KEY(request_id,contract_id)
        REFERENCES contract_signature_requests(request_id,contract_id),
    CONSTRAINT contract_events_actor_fkey FOREIGN KEY(actor_id) REFERENCES users(id),
    CONSTRAINT contract_events_audit_sequence_fkey FOREIGN KEY(audit_sequence)
        REFERENCES audit_events(sequence),
    CHECK(revision BETWEEN 1 AND 4294967295),
    CHECK(kind IN ('created','abandoned','requested','client_signed','rejected',
        'completed','declined','cancelled','expired')),
    CHECK((revision=1)=(kind='created')),
    CHECK((request_id IS NULL)=(kind IN ('created','abandoned'))),
    CHECK((actor_id IS NULL)=(kind='expired')),
    CHECK(kind<>'expired' OR actor_label='system:contract-expiry'),
    CHECK(octet_length(actor_label) BETWEEN 1 AND 1280),
    CHECK(reason IS NULL OR char_length(reason) BETWEEN 1 AND 500),
    CHECK(kind NOT IN ('abandoned','cancelled') OR reason IS NOT NULL),
    CHECK(kind IN ('abandoned','cancelled','rejected','declined') OR reason IS NULL),
    CHECK(operation_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(at_seconds BETWEEN 0 AND 253402300799),
    CHECK(at_nanoseconds BETWEEN 0 AND 999999999),
    CHECK(audit_sequence>=0)
);

CREATE TABLE IF NOT EXISTS contract_party_signatures (
    request_id UUID NOT NULL,
    position SMALLINT NOT NULL,
    operation_id UUID NOT NULL,
    signer_id UUID NOT NULL,
    binding_id UUID NOT NULL,
    leaf_fingerprint BYTEA NOT NULL,
    deployment_id UUID NOT NULL,
    trust_revision BIGINT NOT NULL,
    root_fingerprint BYTEA NOT NULL,
    account_revision BIGINT NOT NULL,
    auth_generation BIGINT NOT NULL,
    actor_email TEXT NOT NULL,
    statement BYTEA NOT NULL,
    statement_digest BYTEA NOT NULL,
    signature BYTEA NOT NULL,
    checked_at BIGINT NOT NULL,
    valid_from BIGINT NOT NULL,
    valid_until BIGINT NOT NULL,
    accepted_at_seconds BIGINT NOT NULL,
    accepted_at_nanoseconds INTEGER NOT NULL,
    timestamp_token BYTEA,
    tsa_chain_pem BYTEA,
    CONSTRAINT contract_signatures_pkey PRIMARY KEY(request_id,position),
    CONSTRAINT contract_signatures_operation_key UNIQUE(operation_id),
    CONSTRAINT contract_signatures_request_fkey FOREIGN KEY(request_id)
        REFERENCES contract_signature_requests(request_id),
    CONSTRAINT contract_signatures_event_fkey FOREIGN KEY(operation_id)
        REFERENCES contract_events(operation_id),
    CONSTRAINT contract_signatures_binding_fkey FOREIGN KEY(binding_id,signer_id,leaf_fingerprint)
        REFERENCES document_signing_bindings(binding_id,signer_id,leaf_fingerprint),
    CONSTRAINT contract_signatures_trust_fkey FOREIGN KEY(deployment_id,trust_revision)
        REFERENCES participant_credential_trust_revisions(deployment_id,revision),
    CHECK(position IN (1,2)),
    CHECK(trust_revision BETWEEN 1 AND 4294967295),
    CHECK(octet_length(root_fingerprint)=32),
    CHECK(octet_length(leaf_fingerprint)=32),
    CHECK(account_revision>=0 AND auth_generation>=0 AND auth_generation<=account_revision),
    CHECK(octet_length(actor_email) BETWEEN 1 AND 1280),
    CHECK(octet_length(statement)=303),
    CHECK(statement_digest=sha256(statement)),
    CHECK(octet_length(signature)=384),
    CHECK(valid_from<=checked_at AND checked_at<=valid_until),
    CHECK(accepted_at_seconds>=checked_at AND accepted_at_seconds<=valid_until),
    CHECK(accepted_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(accepted_at_nanoseconds BETWEEN 0 AND 999999999),
    CHECK((position=1)=(timestamp_token IS NOT NULL)),
    CHECK((timestamp_token IS NULL)=(tsa_chain_pem IS NULL)),
    CHECK(octet_length(timestamp_token) BETWEEN 1 AND 65536),
    CHECK(octet_length(tsa_chain_pem) BETWEEN 1 AND 65536)
);

CREATE TABLE IF NOT EXISTS contract_completions (
    request_id UUID NOT NULL,
    document_token BYTEA NOT NULL,
    manifest BYTEA NOT NULL,
    manifest_sha256 BYTEA NOT NULL,
    manifest_token BYTEA NOT NULL,
    tsa_chain_pem BYTEA NOT NULL,
    completed_at_seconds BIGINT NOT NULL,
    completed_at_nanoseconds INTEGER NOT NULL,
    CONSTRAINT contract_completions_pkey PRIMARY KEY(request_id),
    CONSTRAINT contract_completions_request_fkey FOREIGN KEY(request_id)
        REFERENCES contract_signature_requests(request_id),
    CHECK(octet_length(document_token) BETWEEN 1 AND 65536),
    CHECK(octet_length(manifest) BETWEEN 1 AND 65536),
    CHECK(manifest_sha256=sha256(manifest)),
    CHECK(octet_length(manifest_token) BETWEEN 1 AND 65536),
    CHECK(octet_length(tsa_chain_pem) BETWEEN 1 AND 65536),
    CHECK(completed_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(completed_at_nanoseconds BETWEEN 0 AND 999999999)
);

CREATE TABLE IF NOT EXISTS contract_notifications (
    id UUID NOT NULL,
    request_id UUID NOT NULL,
    contract_id UUID NOT NULL,
    event_revision BIGINT NOT NULL,
    recipient_id UUID NOT NULL,
    kind TEXT NOT NULL,
    message_from TEXT NOT NULL,
    message_to TEXT NOT NULL,
    subject TEXT NOT NULL,
    message_text TEXT NOT NULL,
    created_at_seconds BIGINT NOT NULL,
    delivery_state TEXT NOT NULL,
    delivery_generation BIGINT NOT NULL,
    lease_id UUID,
    lease_until BIGINT,
    provider_deadline BIGINT,
    retry_at BIGINT,
    CONSTRAINT contract_notifications_pkey PRIMARY KEY(id),
    CONSTRAINT contract_notifications_recipient_key UNIQUE(contract_id,event_revision,recipient_id),
    CONSTRAINT contract_notifications_request_fkey FOREIGN KEY(request_id,contract_id)
        REFERENCES contract_signature_requests(request_id,contract_id),
    CONSTRAINT contract_notifications_event_fkey FOREIGN KEY(contract_id,event_revision)
        REFERENCES contract_events(contract_id,revision),
    CONSTRAINT contract_notifications_recipient_fkey FOREIGN KEY(recipient_id) REFERENCES users(id),
    CHECK(id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK(kind IN ('requested','client_signed','rejected','completed','declined','cancelled','expired')),
    CHECK(octet_length(message_from) BETWEEN 3 AND 320),
    CHECK(octet_length(message_to) BETWEEN 3 AND 320),
    CHECK(char_length(subject) BETWEEN 1 AND 200),
    CHECK(char_length(message_text) BETWEEN 1 AND 4096),
    CHECK(created_at_seconds BETWEEN 0 AND 253402300799),
    CHECK(delivery_state IN ('pending','sent','uncertain','rejected')),
    CHECK(delivery_generation>=0),
    CHECK((lease_id IS NULL)=(lease_until IS NULL))
);
CREATE INDEX IF NOT EXISTS contract_notification_deliveries
    ON contract_notifications(delivery_state,retry_at,id);
CREATE INDEX IF NOT EXISTS contract_request_versions
    ON contract_signature_requests(document_id,document_version);
CREATE INDEX IF NOT EXISTS contract_event_requests ON contract_events(request_id,kind);
REVOKE ALL ON contracts,contract_signature_requests,contract_events,contract_party_signatures,
    contract_completions,contract_notifications FROM PUBLIC;

-- The exact CTRTREQ1 and CTRTSIG1 bytes, rebuilt from stored columns.
CREATE OR REPLACE FUNCTION contract_request_bytes(deployment UUID, request UUID, case_id UUID,
    document UUID, version BIGINT, content BYTEA, client UUID, firm_signer UUID, requester UUID,
    client_leaf BYTEA, firm_leaf BYTEA, created_at BIGINT, expires_at BIGINT) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog AS $function$
SELECT convert_to('CTRTREQ1','UTF8') || decode('0101','hex') || uuid_send(deployment)
    || uuid_send(request) || uuid_send(case_id) || uuid_send(document)
    || substring(int8send(version) FROM 5 FOR 4) || content || uuid_send(client)
    || uuid_send(firm_signer) || uuid_send(requester) || client_leaf || firm_leaf
    || int8send(created_at) || int8send(expires_at) || decode('0201','hex')
$function$;

CREATE OR REPLACE FUNCTION contract_party_bytes(deployment UUID, root BYTEA, trust_revision BIGINT,
    account UUID, account_revision BIGINT, auth_generation BIGINT, binding UUID, leaf BYTEA,
    operation UUID, request UUID, request_digest BYTEA, content BYTEA, party SMALLINT,
    signature_digest BYTEA, token_digest BYTEA) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog AS $function$
SELECT convert_to('CTRTSIG1','UTF8') || decode('0101','hex') || uuid_send(deployment) || root
    || substring(int8send(trust_revision) FROM 5 FOR 4) || uuid_send(account)
    || int8send(account_revision) || int8send(auth_generation) || uuid_send(binding) || leaf
    || uuid_send(operation) || uuid_send(request) || request_digest || content
    || substring(int2send(party) FROM 2 FOR 1) || signature_digest || token_digest
$function$;
REVOKE ALL ON FUNCTION contract_request_bytes(UUID,UUID,UUID,UUID,BIGINT,BYTEA,UUID,UUID,UUID,
    BYTEA,BYTEA,BIGINT,BIGINT),
    contract_party_bytes(UUID,BYTEA,BIGINT,UUID,BIGINT,BIGINT,UUID,BYTEA,UUID,UUID,BYTEA,BYTEA,
    SMALLINT,BYTEA,BYTEA) FROM PUBLIC;
