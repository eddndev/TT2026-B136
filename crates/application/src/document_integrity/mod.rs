//! Durable document validation observations and authorized Owner queries.

mod ids;
mod model;
mod port;
mod query;
mod service;
mod validation;

pub use ids::{DocumentIntegrityIncidentId, DocumentIntegrityObservationId};
pub use model::{
    DocumentIntegrityFailure, DocumentIntegrityIncident, DocumentIntegrityObservation,
    DocumentIntegrityReceipt,
};
pub use port::{DocumentIntegrityStore, DocumentIntegrityWorkflow};
pub use query::{DocumentIntegrityPage, DocumentIntegrityQuery};
pub use service::DocumentIntegrityService;
