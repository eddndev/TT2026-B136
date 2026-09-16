//! Exact startup validation of immutable procedural fact history.
mod catalog;
mod constraints;
mod inventory;
mod permissions;

pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};

use application::ApplicationError;
const TABLES: [&str; 2] = ["case_procedural_facts", "case_procedural_fact_revisions"];
const HELPERS: [&str; 11] = [
    "procedural_fact_atom(bytea,integer,text)",
    "procedural_fact_time(bytea,integer,text)",
    "procedural_fact_person(bytea,integer)",
    "procedural_fact_declaration(bytea,integer,text)",
    "procedural_fact_provenance(bytea,integer)",
    "procedural_fact_representation(bytea,integer)",
    "procedural_fact_values(text,bytea)",
    "procedural_fact_source_item(bytea,integer,text)",
    "procedural_fact_sources(bytea)",
    "procedural_fact_submission(bytea)",
    "validate_procedural_fact_sources(uuid,text,jsonb,jsonb)",
];
const TRIGGERS: [&str; 3] = [
    "preserve_procedural_fact_history()",
    "enforce_procedural_fact_root()",
    "enforce_procedural_fact_sequence()",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "procedural fact schema is incomplete; run database migrate with an administrative role"
            .into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("procedural fact schema: {error}"))
}
