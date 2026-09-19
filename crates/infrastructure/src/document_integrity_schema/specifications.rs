pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    (
        "document_integrity_incidents",
        "document_integrity_primary",
        &["id"],
        true,
    ),
    (
        "document_integrity_incidents",
        "document_integrity_observation",
        &["observation_id"],
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
        "document_integrity_incidents",
        "document_integrity_scope",
        &["document_id", "case_id"],
        "document_series",
        &["id", "case_id"],
        false,
    ),
    (
        "document_integrity_incidents",
        "document_integrity_version",
        &["document_id", "document_version"],
        "documents",
        &["id", "version"],
        false,
    ),
    (
        "document_integrity_incidents",
        "document_integrity_requester",
        &["requester"],
        "users",
        &["id"],
        false,
    ),
];
