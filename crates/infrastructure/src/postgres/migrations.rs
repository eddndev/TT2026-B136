//! Versioned schema prerequisites applied before feature migrations.

pub(super) const IDENTITY_MIGRATION: &str =
    include_str!("../../../../migrations/0001_identity.sql");
pub(super) const CASE_MIGRATION: &str = include_str!("../../../../migrations/0002_cases.sql");
pub(super) const DOCUMENT_MIGRATION: &str =
    include_str!("../../../../migrations/0003_case_documents_audit.sql");
pub(super) const VERSION_MIGRATION: &str =
    include_str!("../../../../migrations/0004_document_versions.sql");
pub(super) const METADATA_MIGRATION: &str =
    include_str!("../../../../migrations/0005_document_metadata.sql");
pub(super) const PARTICIPANT_MIGRATION: &str =
    include_str!("../../../../migrations/0006_case_participants.sql");
pub(super) const CASE_ADMINISTRATION_MIGRATION: &str =
    include_str!("../../../../migrations/0007_case_administration.sql");
pub(super) const CASE_STAGE_MIGRATIONS: [&str; 3] = [
    include_str!("../../../../migrations/0008_case_stages.sql"),
    include_str!("../../../../migrations/0008_case_stage_values.sql"),
    include_str!("../../../../migrations/0008_case_stage_guards.sql"),
];
pub(super) const CREDENTIAL_TRUST_MIGRATION: &str =
    include_str!("../../../../migrations/0009_participant_credential_trust.sql");
pub(super) const TYPED_PARTICIPANT_MIGRATIONS: [&str; 6] = [
    include_str!("../../../../migrations/0010_typed_values_primitives.sql"),
    include_str!("../../../../migrations/0010_typed_values.sql"),
    include_str!("../../../../migrations/0010_typed_participants.sql"),
    include_str!("../../../../migrations/0010_typed_guards.sql"),
    include_str!("../../../../migrations/0010_typed_reviews.sql"),
    include_str!("../../../../migrations/0010_typed_credentials.sql"),
];
pub(super) const HEARING_MIGRATIONS: [&str; 4] = [
    include_str!("../../../../migrations/0011_hearings_values.sql"),
    include_str!("../../../../migrations/0011_hearings_receipts.sql"),
    include_str!("../../../../migrations/0011_hearings.sql"),
    include_str!("../../../../migrations/0011_hearings_guards.sql"),
];
pub(super) const HEARING_RESULT_MIGRATIONS: [&str; 5] = [
    include_str!("../../../../migrations/0012_hearing_results_time.sql"),
    include_str!("../../../../migrations/0012_hearing_results_values.sql"),
    include_str!("../../../../migrations/0012_hearing_results_receipts.sql"),
    include_str!("../../../../migrations/0012_hearing_results.sql"),
    include_str!("../../../../migrations/0012_hearing_results_guards.sql"),
];
pub(super) const JUDICIAL_CALENDAR_MIGRATIONS: [&str; 6] = [
    include_str!("../../../../migrations/0013_judicial_calendar_primitives.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_sources.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_values.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_receipts.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_tables.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_guards.sql"),
];
pub(super) const PROCEDURAL_FACT_MIGRATIONS: [&str; 11] = [
    include_str!("../../../../migrations/0014_procedural_fact_primitives.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_time.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_people.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_provenance.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_values.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_source_items.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_sources.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_receipts.sql"),
    include_str!("../../../../migrations/0014_procedural_facts.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_source_guards.sql"),
    include_str!("../../../../migrations/0014_procedural_facts_guards.sql"),
];
pub(super) const DEADLINE_SOURCE_EVENT_MIGRATION: &str =
    include_str!("../../../../migrations/0015_deadline_source_events.sql");
pub(super) const DEADLINE_PROFILE_MIGRATIONS: &[&str] = &[
    include_str!("../../../../migrations/0016_deadline_profile_projection.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_receipts.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_tables.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_guards.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_events.sql"),
];
