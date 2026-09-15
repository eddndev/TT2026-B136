//! Authenticated case-local participant directory with audited revision history.

mod action;
mod model;
mod port;
mod query;
mod service;

pub use action::ParticipantAction;
pub use domain::participants::{
    DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantValues,
};
pub use model::{
    participant_digest, ParticipantActorSnapshot, ParticipantHistoryPage, ParticipantPage,
    ParticipantSnapshot,
};
pub use port::{ParticipantStore, ParticipantWorkflow};
pub use query::{ParticipantHistoryQuery, ParticipantQuery, ParticipantStatusFilter};
pub use service::ParticipantService;
