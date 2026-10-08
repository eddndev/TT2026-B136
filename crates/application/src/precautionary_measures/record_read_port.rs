use super::*;
use crate::{identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId},
};

/// Reauthorize the full active principal and case membership under the shared
/// audit lock before lookup, including reads on closed cases. Validate durable
/// origins and exact source existence independently of supplied receipt validity.
/// Commit the read audit before returning the original complete group and closure.
pub trait MeasureDecisionRecordReadStore: Send + Sync {
    /// Select immutable decisions, not current measure heads, in ascending UUID
    /// order strictly after the exclusive cursor. Probe at most limit + 1 roots;
    /// return at most limit items. Continuation requires a full page and its last ID.
    fn list(
        &self,
        actor: &Principal,
        case_id: CaseId,
        query: MeasureDecisionReadQuery,
    ) -> Result<MeasureDecisionRecordPage, ApplicationError>;
    /// Return the exact immutable decision and its original dependencies.
    fn get(
        &self,
        actor: &Principal,
        case_id: CaseId,
        decision: MeasureDecisionId,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError>;
    /// An authorized absence is NotFound; an inconsistent or incomplete origin
    /// is an integrity error. Never replace an original group with later heads.
    fn get_operation(
        &self,
        actor: &Principal,
        case_id: CaseId,
        operation: MeasureDecisionOperationId,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError>;
}

pub trait MeasureDecisionRecordReadWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: MeasureDecisionReadQuery,
    ) -> Result<MeasureDecisionRecordPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        decision: MeasureDecisionId,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError>;
    fn get_operation(
        &self,
        token: &str,
        case_id: CaseId,
        operation: MeasureDecisionOperationId,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError>;
}
