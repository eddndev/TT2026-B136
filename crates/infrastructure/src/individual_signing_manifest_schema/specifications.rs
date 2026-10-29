use super::{RECEIPTS, TABLES};

pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    (
        TABLES[0],
        "individual_document_manifests_pkey",
        &["operation_id"],
        true,
    ),
    (
        TABLES[1],
        "individual_manifest_cutoff_pkey",
        &["singleton"],
        true,
    ),
];

type ForeignKey = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static [&'static str],
);

pub(super) const FOREIGN_KEYS: &[ForeignKey] = &[(
    TABLES[0],
    "individual_manifest_receipt_fkey",
    &["operation_id"],
    RECEIPTS,
    &["operation_id"],
)];
