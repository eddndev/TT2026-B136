use super::*;
use crate::{identity::Principal, ApplicationError};
use domain::cases::CaseId;
use domain::precautionary_hearings::{
    PrecautionaryHearingId, PrecautionaryHearingOperationId, PrecautionaryHearingRevision,
};

/// Reauthorize the full active principal and case membership under the shared
/// audit lock before lookup, including reads on closed cases. Validate the durable
/// origin, current head and source existence separately from supplied receipt
/// validity. Commit the read audit before returning complete historical evidence.
pub trait PrecautionaryHearingReadStore: Send + Sync {
    /// Select current heads before pagination, strictly after the exclusive UUID
    /// cursor. Probe at most limit + 1 roots; return at most limit items. A further
    /// page requires a full page and its last ID as continuation; otherwise None.
    fn list(
        &self,
        actor: &Principal,
        case_id: CaseId,
        query: PrecautionaryHearingReadQuery,
    ) -> Result<PrecautionaryHearingPage, ApplicationError>;
    /// None selects the current head. Some selects exactly that revision, with
    /// its full original prefix and dependency closure. Never substitute a head.
    fn get(
        &self,
        actor: &Principal,
        case_id: CaseId,
        hearing: PrecautionaryHearingId,
        revision: Option<PrecautionaryHearingRevision>,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
    /// Return the original operation, even if later revisions exist. An authorized
    /// absence is NotFound; a corrupt/incomplete origin is an integrity error.
    fn get_operation(
        &self,
        actor: &Principal,
        case_id: CaseId,
        operation: PrecautionaryHearingOperationId,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
}

pub trait PrecautionaryHearingReadWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: PrecautionaryHearingReadQuery,
    ) -> Result<PrecautionaryHearingPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        hearing: PrecautionaryHearingId,
        revision: Option<PrecautionaryHearingRevision>,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
    fn get_operation(
        &self,
        token: &str,
        case_id: CaseId,
        operation: PrecautionaryHearingOperationId,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
}
