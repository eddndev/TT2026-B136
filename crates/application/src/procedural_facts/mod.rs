//! Declared procedural facts with stable roots and correctable exact sources.

mod administration;
mod base;
mod command;
mod error;
mod material;
mod model;
mod participants;
mod port;
mod prepared;
mod query;
mod selection;

pub use administration::validate_fact_administration;
pub use base::validate_fact_base;
pub use command::{FactChange, NotificationCommand, ProceduralFactCommand, ResolutionCommand};
pub use domain::procedural_facts::*;
pub use error::ProceduralFactError;
pub use material::*;
pub use model::*;
pub use participants::{resolve_fact_participants, FactParticipantProjection};
pub use port::{FactPreparation, ProceduralFactStore, ProceduralFactWorkflow};
pub use prepared::PreparedFactChange;
pub use query::{
    FactHistoryQuery, FactListQuery, FactStatusFilter, NotificationQuery, ResolutionQuery,
};
pub use selection::FactSourceSelection;

mod hearings;
mod source_canonical;
mod source_encoding;
mod source_shape;
pub use hearings::{resolve_fact_hearings, FactHearingProjection};
pub use source_canonical::{fact_sources_bytes, fact_sources_digest};

mod receipt;
mod submission;
pub use receipt::{
    fact_history_receipt_matches, fact_receipt_matches, fact_snapshot_receipt_matches,
};
pub use submission::{fact_submission_bytes, fact_submission_digest, fact_values_digest};

mod resolution_source;
pub use resolution_source::{resolve_fact_resolution, FactResolutionProjection};

mod preparation;
mod service;
pub use service::ProceduralFactService;

mod service_workflow;
