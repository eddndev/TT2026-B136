use super::*;
use crate::{identity::Principal, ApplicationError};
use domain::{cases::CaseId, precautionary_measures::MeasureCorrectionOperationId};

/// Reauthorize the current full staff principal and case access under the audit
/// lock, including closed cases. Reconstruct the original receipt and durable
/// sources; commit its access audit before disclosure. Fresh target-head and
/// no-dependant mutation admission must not retroactively reject historical reads.
pub trait MeasureAdministrativeReadStore: Send + Sync {
    /// Probe at most limit + 1 immutable operations in ascending UUID order after
    /// the exclusive cursor. Return at most limit entries with complete closures.
    fn list(
        &self,
        actor: &Principal,
        case_id: CaseId,
        query: MeasureAdministrativeReadQuery,
    ) -> Result<MeasureAdministrativePage, ApplicationError>;
    /// Authorized absence is NotFound; lost payload or audit evidence is an
    /// integrity error. Never replace an exact operation with a newer head.
    fn get_operation(
        &self,
        actor: &Principal,
        case_id: CaseId,
        operation: MeasureCorrectionOperationId,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError>;
}

pub trait MeasureAdministrativeReadWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: MeasureAdministrativeReadQuery,
    ) -> Result<MeasureAdministrativePage, ApplicationError>;
    fn get_operation(
        &self,
        token: &str,
        case_id: CaseId,
        operation: MeasureCorrectionOperationId,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError>;
}
