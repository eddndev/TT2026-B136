use super::TABLES;

pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    (
        TABLES[0],
        "individual_document_signatures_pkey",
        &["operation_id"],
        true,
    ),
    (
        TABLES[0],
        "individual_document_signatures_audit_sequence_key",
        &["audit_sequence"],
        false,
    ),
    (
        TABLES[0],
        "individual_document_signatures_document_id_document_version_key",
        &["document_id", "document_version"],
        false,
    ),
];

type ForeignKey = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static [&'static str],
    bool,
);

pub(super) const FOREIGN_KEYS: &[ForeignKey] = &[
    (
        TABLES[0],
        "individual_signature_binding_fkey",
        &["binding_id", "signer_id", "leaf_fingerprint"],
        "document_signing_bindings",
        &["binding_id", "signer_id", "leaf_fingerprint"],
        false,
    ),
    (
        TABLES[0],
        "individual_signature_document_fkey",
        &["document_id", "document_version"],
        "documents",
        &["id", "version"],
        false,
    ),
    (
        TABLES[0],
        "individual_signature_case_fkey",
        &["document_id", "case_id"],
        "document_series",
        &["id", "case_id"],
        false,
    ),
    (
        TABLES[0],
        "individual_signature_trust_fkey",
        &["deployment_id", "trust_revision"],
        "participant_credential_trust_revisions",
        &["deployment_id", "revision"],
        false,
    ),
    (
        TABLES[0],
        "individual_signature_audit_fkey",
        &["audit_sequence"],
        "audit_events",
        &["sequence"],
        false,
    ),
];
