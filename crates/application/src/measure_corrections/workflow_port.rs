use super::{
    MeasureAdministrativeCommand, MeasureAdministrativeConfirmation,
    MeasureAdministrativePreparation, MeasureAdministrativeReview,
    MeasureAdministrativeStoredOperation, PreparedMeasureAdministrative,
};
use crate::{documents::StageSupportReadLimits, identity::Principal, ApplicationError};
use domain::cases::CaseId;

pub trait MeasureAdministrativeStore: Send + Sync {
    /// Reauthorize the complete current principal and case access before looking
    /// up an operation. Exact replay remains available on a closed case. Fresh
    /// work requires active exact context, the current Valid target head and a
    /// complete bounded dependency inventory, including hearing prefixes and
    /// decisions without measure rows. Resolve retained historical sources and
    /// the last actual judicial support without replacing them with current heads.
    fn prepare(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: &MeasureAdministrativeCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<MeasureAdministrativePreparation, ApplicationError>;

    /// Under the shared audit lock recheck current principal, case access,
    /// active context, exact Valid target head and complete durable absence of
    /// dependants. Prevent new dependencies from racing with this check. Compare
    /// the admitted encrypted support and exact retained sources without parsing
    /// or decrypting inside the lock. Commit receipt, record, origin, operation,
    /// new head and one audit event atomically. An exact raced operation returns
    /// its original receipt and ancestor closure; any rejected mutation writes
    /// nothing. Unrelated inventory changes do not alone invalidate the review.
    fn commit(
        &self,
        actor: &Principal,
        case_id: CaseId,
        prepared: PreparedMeasureAdministrative,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError>;
}

pub trait MeasureAdministrativeWorkflow: Send + Sync {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureAdministrativeCommand,
    ) -> Result<MeasureAdministrativeReview, ApplicationError>;

    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureAdministrativeCommand,
        confirmation: MeasureAdministrativeConfirmation,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError>;
}
