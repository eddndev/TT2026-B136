use super::TABLES;

pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    (
        TABLES[0],
        "owner_certificate_registrations_pkey",
        &["binding_id"],
        true,
    ),
    (
        TABLES[0],
        "owner_certificate_registrations_audit_sequence_key",
        &["audit_sequence"],
        false,
    ),
    (
        TABLES[1],
        "owner_certificate_withdrawals_pkey",
        &["binding_id"],
        true,
    ),
    (
        TABLES[1],
        "owner_certificate_withdrawals_audit_sequence_key",
        &["audit_sequence"],
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
        "owner_certificate_registrations_owner_id_fkey",
        &["owner_id"],
        "users",
        &["id"],
        false,
    ),
    (
        TABLES[0],
        "owner_certificate_registrations_trust_fkey",
        &["deployment_id", "trust_revision"],
        "participant_credential_trust_revisions",
        &["deployment_id", "revision"],
        false,
    ),
    (
        TABLES[0],
        "owner_certificate_registrations_audit_sequence_fkey",
        &["audit_sequence"],
        "audit_events",
        &["sequence"],
        false,
    ),
    (
        TABLES[1],
        "owner_certificate_withdrawals_binding_id_fkey",
        &["binding_id"],
        TABLES[0],
        &["binding_id"],
        false,
    ),
    (
        TABLES[1],
        "owner_certificate_withdrawals_audit_sequence_fkey",
        &["audit_sequence"],
        "audit_events",
        &["sequence"],
        false,
    ),
];
