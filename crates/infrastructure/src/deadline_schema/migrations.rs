//! Installation order shared with the effective-function verifier.
pub(crate) const SQL: &[&str] = &[
    include_str!("../../../../migrations/0017_deadline_input_selection.sql"),
    include_str!("../../../../migrations/0017_deadline_attention.sql"),
    include_str!("../../../../migrations/0017_deadline_receipts.sql"),
    include_str!("../../../../migrations/0017_deadline_tables.sql"),
    include_str!("../../../../migrations/0017_deadline_guards.sql"),
    include_str!("../../../../migrations/0018_deadline_submission_v2.sql"),
    include_str!("../../../../migrations/0018_deadline_receipts.sql"),
    include_str!("../../../../migrations/0018_deadline_observations.sql"),
    include_str!("../../../../migrations/0018_deadline_tracking.sql"),
    include_str!("../../../../migrations/0018_deadline_tracking_consistency.sql"),
    include_str!("../../../../migrations/0018_deadline_tracking_columns.sql"),
    include_str!("../../../../migrations/0018_deadline_tracking_constraints.sql"),
    include_str!("../../../../migrations/0018_deadline_guards.sql"),
];
