pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    (
        "document_upload_origins",
        "document_upload_origin_primary",
        &["document_id"],
        true,
    ),
    (
        "document_upload_origins",
        "document_upload_origin_audit_unique",
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
        "document_upload_origins",
        "document_upload_origin_scope",
        &["document_id", "case_id"],
        "document_series",
        &["id", "case_id"],
        false,
    ),
    (
        "document_upload_origins",
        "document_upload_origin_actor",
        &["actor_id"],
        "users",
        &["id"],
        false,
    ),
    (
        "document_upload_origins",
        "document_upload_origin_audit",
        &["audit_sequence"],
        "audit_events",
        &["sequence"],
        false,
    ),
];
