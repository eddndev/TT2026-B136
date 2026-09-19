pub(super) const KEYS: &[(&str, &str, &[&str], bool)] = &[
    ("users", "users_pkey", &["id"], true),
    ("users", "users_email_key", &["email"], false),
    (
        "case_memberships",
        "case_memberships_pkey",
        &["case_id", "user_id"],
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
        "case_memberships",
        "case_memberships_case_id_fkey",
        &["case_id"],
        "cases",
        &["id"],
        false,
    ),
    (
        "case_memberships",
        "case_memberships_user_id_fkey",
        &["user_id"],
        "users",
        &["id"],
        false,
    ),
];
