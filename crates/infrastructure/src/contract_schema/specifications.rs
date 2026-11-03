/// (table, constraint and index name, ordered columns, primary)
pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    ("contracts", "contracts_pkey", &["id"], true),
    (
        "contracts",
        "contracts_document_key",
        &["document_id"],
        false,
    ),
    (
        "contracts",
        "contracts_identity_key",
        &["id", "case_id", "document_id"],
        false,
    ),
    (
        "contract_signature_requests",
        "contract_requests_pkey",
        &["request_id"],
        true,
    ),
    (
        "contract_signature_requests",
        "contract_requests_contract_key",
        &["request_id", "contract_id"],
        false,
    ),
    (
        "contract_events",
        "contract_events_pkey",
        &["contract_id", "revision"],
        true,
    ),
    (
        "contract_events",
        "contract_events_operation_key",
        &["operation_id"],
        false,
    ),
    (
        "contract_events",
        "contract_events_audit_sequence_key",
        &["audit_sequence"],
        false,
    ),
    (
        "contract_party_signatures",
        "contract_signatures_pkey",
        &["request_id", "position"],
        true,
    ),
    (
        "contract_party_signatures",
        "contract_signatures_operation_key",
        &["operation_id"],
        false,
    ),
    (
        "contract_completions",
        "contract_completions_pkey",
        &["request_id"],
        true,
    ),
    (
        "contract_notifications",
        "contract_notifications_pkey",
        &["id"],
        true,
    ),
    (
        "contract_notifications",
        "contract_notifications_recipient_key",
        &["contract_id", "event_revision", "recipient_id"],
        false,
    ),
];

/// (table, index that backs no constraint, ordered columns)
pub(super) const LOOKUPS: &[(&str, &str, &[&str])] = &[
    (
        "contract_notifications",
        "contract_notification_deliveries",
        &["delivery_state", "retry_at", "id"],
    ),
    (
        "contract_signature_requests",
        "contract_request_versions",
        &["document_id", "document_version"],
    ),
    (
        "contract_events",
        "contract_event_requests",
        &["request_id", "kind"],
    ),
];

type ForeignKey = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static [&'static str],
);

const BINDING: &[&str] = &["binding_id", "signer_id", "leaf_fingerprint"];

/// (table, constraint, ordered columns, referenced table, referenced columns)
pub(super) const FOREIGN_KEYS: &[ForeignKey] = &[
    (
        "contracts",
        "contracts_series_fkey",
        &["document_id", "case_id"],
        "document_series",
        &["id", "case_id"],
    ),
    (
        "contracts",
        "contracts_creator_fkey",
        &["created_by"],
        "users",
        &["id"],
    ),
    (
        "contract_signature_requests",
        "contract_requests_contract_fkey",
        &["contract_id", "case_id", "document_id"],
        "contracts",
        &["id", "case_id", "document_id"],
    ),
    (
        "contract_signature_requests",
        "contract_requests_document_fkey",
        &["document_id", "document_version"],
        "documents",
        &["id", "version"],
    ),
    (
        "contract_signature_requests",
        "contract_requests_client_binding_fkey",
        &["client_binding_id", "client_id", "client_leaf"],
        "document_signing_bindings",
        BINDING,
    ),
    (
        "contract_signature_requests",
        "contract_requests_firm_binding_fkey",
        &["firm_binding_id", "firm_signer_id", "firm_leaf"],
        "document_signing_bindings",
        BINDING,
    ),
    (
        "contract_signature_requests",
        "contract_requests_requester_fkey",
        &["requester_id"],
        "users",
        &["id"],
    ),
    (
        "contract_signature_requests",
        "contract_requests_subject_fkey",
        &["client_subject_id", "client_subject_revision"],
        "case_subject_revisions",
        &["subject_id", "revision"],
    ),
    (
        "contract_events",
        "contract_events_contract_fkey",
        &["contract_id"],
        "contracts",
        &["id"],
    ),
    (
        "contract_events",
        "contract_events_request_fkey",
        &["request_id", "contract_id"],
        "contract_signature_requests",
        &["request_id", "contract_id"],
    ),
    (
        "contract_events",
        "contract_events_actor_fkey",
        &["actor_id"],
        "users",
        &["id"],
    ),
    (
        "contract_events",
        "contract_events_audit_sequence_fkey",
        &["audit_sequence"],
        "audit_events",
        &["sequence"],
    ),
    (
        "contract_party_signatures",
        "contract_signatures_request_fkey",
        &["request_id"],
        "contract_signature_requests",
        &["request_id"],
    ),
    (
        "contract_party_signatures",
        "contract_signatures_event_fkey",
        &["operation_id"],
        "contract_events",
        &["operation_id"],
    ),
    (
        "contract_party_signatures",
        "contract_signatures_binding_fkey",
        BINDING,
        "document_signing_bindings",
        BINDING,
    ),
    (
        "contract_party_signatures",
        "contract_signatures_trust_fkey",
        &["deployment_id", "trust_revision"],
        "participant_credential_trust_revisions",
        &["deployment_id", "revision"],
    ),
    (
        "contract_completions",
        "contract_completions_request_fkey",
        &["request_id"],
        "contract_signature_requests",
        &["request_id"],
    ),
    (
        "contract_notifications",
        "contract_notifications_request_fkey",
        &["request_id", "contract_id"],
        "contract_signature_requests",
        &["request_id", "contract_id"],
    ),
    (
        "contract_notifications",
        "contract_notifications_event_fkey",
        &["contract_id", "event_revision"],
        "contract_events",
        &["contract_id", "revision"],
    ),
    (
        "contract_notifications",
        "contract_notifications_recipient_fkey",
        &["recipient_id"],
        "users",
        &["id"],
    ),
];

/// Deferred constraint triggers, each counted as a constraint of its table.
pub(super) const PAIRS: [(&str, &str); 3] = [
    ("contracts", "contract_root_pair"),
    ("contract_signature_requests", "contract_request_pair"),
    ("contract_events", "contract_completion_pair"),
];
