//! Authenticated case workflows and actor-scoped transactional persistence.

use domain::cases::{CaseId, CaseMetadata};
use domain::clock::OffsetDateTime;
use domain::identity::UserId;

use super::{
    CaseAdministrationDetail, CaseAdministrationHistoryPage, CaseAdministrationHistoryQuery,
    CaseAdministrationPage, CaseAdministrationQuery, CaseAdministrativeStatus, CaseEditableValues,
    CaseRecord, CaseRevisionExpectation, PenalCaseCreation,
};
use crate::ApplicationError;

/// Authenticates every operation; basic client views remain separate from staff data.
pub trait CaseWorkflow: Send + Sync {
    fn create(
        &self,
        token: &str,
        title: &str,
        reference: &str,
    ) -> Result<CaseRecord, ApplicationError>;
    fn list(
        &self,
        token: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError>;
    fn get(&self, token: &str, id: CaseId) -> Result<CaseRecord, ApplicationError>;
    fn assign(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError>;
    fn remove(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError>;
    fn register_penal(
        &self,
        token: &str,
        creation: PenalCaseCreation,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn replace_administration(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseRevisionExpectation,
        values: CaseEditableValues,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn change_administrative_status(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseRevisionExpectation,
        status: CaseAdministrativeStatus,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn list_administrations(
        &self,
        token: &str,
        query: CaseAdministrationQuery,
    ) -> Result<CaseAdministrationPage, ApplicationError>;
    fn get_administration(
        &self,
        token: &str,
        id: CaseId,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn administration_history(
        &self,
        token: &str,
        id: CaseId,
        query: CaseAdministrationHistoryQuery,
    ) -> Result<CaseAdministrationHistoryPage, ApplicationError>;
}

/// Revalidates the active actor, role and scope inside the common audited transaction.
///
/// Reads commit an event before returning data. Mutations return their own
/// confirmed projection, including captured author and time. A baseline is not
/// an invented historical revision. Status changes preserve current text and
/// profile within the transaction; replace never changes status or stage.
pub trait CaseRepository: Send + Sync {
    fn create_basic(
        &self,
        actor: UserId,
        id: CaseId,
        metadata: CaseMetadata,
        at: OffsetDateTime,
    ) -> Result<CaseRecord, ApplicationError>;
    fn list_basic(
        &self,
        actor: UserId,
        limit: u32,
        offset: u32,
        at: OffsetDateTime,
    ) -> Result<Vec<CaseRecord>, ApplicationError>;
    fn get_basic(
        &self,
        actor: UserId,
        id: CaseId,
        at: OffsetDateTime,
    ) -> Result<CaseRecord, ApplicationError>;
    fn add_member(
        &self,
        id: CaseId,
        user_id: UserId,
        actor: UserId,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
    fn remove_member(
        &self,
        id: CaseId,
        user_id: UserId,
        actor: UserId,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
    fn register_penal(
        &self,
        actor: UserId,
        id: CaseId,
        creation: PenalCaseCreation,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn replace_administration(
        &self,
        actor: UserId,
        id: CaseId,
        expected: CaseRevisionExpectation,
        values: CaseEditableValues,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn change_administrative_status(
        &self,
        actor: UserId,
        id: CaseId,
        expected: CaseRevisionExpectation,
        status: CaseAdministrativeStatus,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn list_administrations(
        &self,
        actor: UserId,
        query: CaseAdministrationQuery,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationPage, ApplicationError>;
    fn get_administration(
        &self,
        actor: UserId,
        id: CaseId,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn administration_history(
        &self,
        actor: UserId,
        id: CaseId,
        query: CaseAdministrationHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationHistoryPage, ApplicationError>;
}
