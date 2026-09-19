//! Ordered SQL also supplies the expected effective function definitions.
pub(crate) const SQL: &[&str] = &[
    include_str!("../../../../migrations/0019_deadline_dispatch_tables.sql"),
    include_str!("../../../../migrations/0019_deadline_dispatch_candidates.sql"),
    include_str!("../../../../migrations/0019_deadline_dispatch_job_guards.sql"),
    include_str!("../../../../migrations/0019_deadline_dispatch_cursor_guards.sql"),
];
