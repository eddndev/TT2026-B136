//! Installation order and effective function sources are kept separate.
pub(crate) const SQL: &[&str] = &[
    include_str!("../../../../migrations/0020_deadline_worker_tables.sql"),
    include_str!("../../../../migrations/0020_deadline_worker_provenance.sql"),
    include_str!("../../../../migrations/0020_deadline_worker_observations.sql"),
    include_str!("../../../../migrations/0020_deadline_worker_transition.sql"),
    include_str!("../../../../migrations/0020_deadline_worker_sequence.sql"),
    include_str!("../../../../migrations/0020_deadline_worker_result_guards.sql"),
    include_str!("../../../../migrations/0020_deadline_worker_attempt_guards.sql"),
];
