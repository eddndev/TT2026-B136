pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    ("case_report_jobs", "case_report_jobs_pkey", &["id"], true),
    (
        "case_report_jobs",
        "case_report_jobs_requester_id_operation_id_key",
        &["requester_id", "operation_id"],
        false,
    ),
    (
        "case_report_snapshots",
        "case_report_snapshots_pkey",
        &["report_id"],
        true,
    ),
    (
        "case_report_artifacts",
        "case_report_artifacts_pkey",
        &["report_id", "format"],
        true,
    ),
    (
        "case_report_notices",
        "case_report_notices_pkey",
        &["report_id"],
        true,
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
        "case_report_jobs",
        "case_report_jobs_requester_id_fkey",
        &["requester_id"],
        "users",
        &["id"],
        false,
    ),
    (
        "case_report_snapshots",
        "case_report_snapshots_report_id_fkey",
        &["report_id"],
        "case_report_jobs",
        &["id"],
        false,
    ),
    (
        "case_report_artifacts",
        "case_report_artifacts_report_id_fkey",
        &["report_id"],
        "case_report_snapshots",
        &["report_id"],
        false,
    ),
    (
        "case_report_notices",
        "case_report_notices_report_id_fkey",
        &["report_id"],
        "case_report_jobs",
        &["id"],
        false,
    ),
];
