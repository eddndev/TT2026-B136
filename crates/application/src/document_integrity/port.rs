use super::{
    DocumentIntegrityIncident, DocumentIntegrityIncidentId, DocumentIntegrityObservation,
    DocumentIntegrityPage, DocumentIntegrityQuery, DocumentIntegrityReceipt,
};
use crate::ApplicationError;
use domain::{clock::OffsetDateTime, identity::UserId};

pub trait DocumentIntegrityStore: Send + Sync {
    /// Commits an observation, the durable Owner-visible incident and rejection audit together.
    /// Exact observation replay returns its first receipt; changed reuse is a conflict.
    /// A later requester revocation must not discard an already authorized technical observation.
    /// Persistence never records the vault, plaintext, token or keys in the incident.
    fn record_rejection(
        &self,
        observation: &DocumentIntegrityObservation,
    ) -> Result<DocumentIntegrityReceipt, ApplicationError>;

    /// Requires a currently active Owner and commits query audit before returning a page.
    /// Order is strictly ascending UUID; filter and limit apply inside that transaction.
    fn list(
        &self,
        actor: UserId,
        query: DocumentIntegrityQuery,
        at: OffsetDateTime,
    ) -> Result<DocumentIntegrityPage, ApplicationError>;

    /// Requires a currently active Owner and commits access audit before returning a record.
    fn get(
        &self,
        actor: UserId,
        id: DocumentIntegrityIncidentId,
        at: OffsetDateTime,
    ) -> Result<DocumentIntegrityIncident, ApplicationError>;
}

pub trait DocumentIntegrityWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        query: DocumentIntegrityQuery,
    ) -> Result<DocumentIntegrityPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        id: DocumentIntegrityIncidentId,
    ) -> Result<DocumentIntegrityIncident, ApplicationError>;
}
