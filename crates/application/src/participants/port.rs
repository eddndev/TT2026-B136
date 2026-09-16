use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::identity::UserId;

use super::{
    DirectoryStatus, ParticipantDetail, ParticipantHistoryPage, ParticipantHistoryQuery,
    ParticipantId, ParticipantPage, ParticipantQuery, ParticipantRevision, ParticipantSnapshot,
    ParticipantValues,
};
use crate::ApplicationError;

/// Every operation authenticates its bearer identity before using storage.
pub trait ParticipantWorkflow: Send + Sync {
    fn create(
        &self,
        token: &str,
        case_id: CaseId,
        display_name: &str,
        procedural_role: &str,
        organization: Option<&str>,
        legal_status: Option<&str>,
    ) -> Result<ParticipantSnapshot, ApplicationError>;
    fn replace(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        expected_revision: ParticipantRevision,
        values: ParticipantValues,
    ) -> Result<ParticipantSnapshot, ApplicationError>;
    fn change_status(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        expected_revision: ParticipantRevision,
        status: DirectoryStatus,
    ) -> Result<ParticipantDetail, ApplicationError>;
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: ParticipantQuery,
    ) -> Result<ParticipantPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
    ) -> Result<ParticipantDetail, ApplicationError>;
    fn get_revision(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
    ) -> Result<ParticipantDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        query: ParticipantHistoryQuery,
    ) -> Result<ParticipantHistoryPage, ApplicationError>;
}

/// Revalidates active actor, role, case and membership inside each transaction.
///
/// All methods hold the common audit lock and commit an event before returning
/// participant data. Writes return their committed snapshot and captured actor,
/// never a subsequent read or a reconstruction from the caller's identity.
pub trait ParticipantStore: Send + Sync {
    /// Creates an immutable root and active revision one with its event.
    fn create(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ParticipantId,
        values: ParticipantValues,
        at: OffsetDateTime,
    ) -> Result<ParticipantSnapshot, ApplicationError>;
    /// Requires the current revision to match, then appends the complete values.
    fn replace(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ParticipantId,
        expected_revision: ParticipantRevision,
        values: ParticipantValues,
        at: OffsetDateTime,
    ) -> Result<ParticipantSnapshot, ApplicationError>;
    /// Reads current values and changes only their organizational state atomically.
    fn change_status(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ParticipantId,
        expected_revision: ParticipantRevision,
        status: DirectoryStatus,
        at: OffsetDateTime,
    ) -> Result<ParticipantDetail, ApplicationError>;
    /// Selects latest revisions and filters before stable, exclusive UUID pagination.
    fn list(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: ParticipantQuery,
        at: OffsetDateTime,
    ) -> Result<ParticipantPage, ApplicationError>;
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ParticipantId,
        at: OffsetDateTime,
    ) -> Result<ParticipantDetail, ApplicationError>;
    fn get_revision(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
        at: OffsetDateTime,
    ) -> Result<ParticipantDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ParticipantId,
        query: ParticipantHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ParticipantHistoryPage, ApplicationError>;
}
