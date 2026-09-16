//! Startup protection for represented identities and the typed history union.

mod catalog;
mod constraints;
mod inventory;
mod permissions;

pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use inventory::validate_credential_binding;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};

use application::ApplicationError;

const TABLES: [&str; 6] = [
    "case_subjects",
    "case_subject_revisions",
    "case_participant_typed_revisions",
    "subject_identity_reviews",
    "participant_identity_reviews",
    "participant_credential_evidence",
];

const HELPERS: [&str; 8] = [
    "typed_u32(bytea,integer)",
    "typed_read(bytea,integer,text)",
    "typed_subject_values(bytea)",
    "typed_participant_values(bytea)",
    "typed_review_values(bytea)",
    "typed_directory_stamp(uuid,uuid,bigint,uuid,bigint)",
    "typed_actor_guard(uuid,uuid,text)",
    "typed_support_guard(uuid,jsonb)",
];
const TRIGGERS: [&str; 6] = [
    "preserve_typed_history()",
    "enforce_subject_sequence()",
    "enforce_typed_sequence()",
    "typed_root_complete()",
    "typed_review_guard()",
    "typed_credential_guard()",
];

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "typed participant schema is incomplete; run database migrate with an administrative role"
            .into(),
    )
}
fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "typed participant inventory is inconsistent; restore a consistent database".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("typed participant schema: {error}"))
}
