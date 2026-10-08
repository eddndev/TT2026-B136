use super::{MeasureRecordDetail, MeasureRecordPage, MeasureRecordReadQuery};
use crate::{identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    precautionary_hearings::{MeasureId, PrecautionaryMeasureRef},
};

/// Reauthorize the full current staff principal and case access under the audit
/// lock, including closed cases. Verify durable owner origins, complete siblings,
/// source existence and mutation audits before committing the access audit.
/// Historical reads grant no fresh mutation or target eligibility.
pub trait MeasureRecordReadStore: Send + Sync {
    /// Probe at most limit + 1 roots after the exclusive measure UUID cursor.
    /// Return at most limit actual heads, including terminal and erroneous
    /// captures. Validate each complete proof with its existing independent
    /// bounds; there is no additional page-wide owner or row budget.
    fn list(
        &self,
        actor: &Principal,
        case_id: CaseId,
        query: MeasureRecordReadQuery,
    ) -> Result<MeasureRecordPage, ApplicationError>;

    /// Select the actual highest revision across judicial and administrative
    /// families before applying field bounds. Missing or malformed advertised
    /// latest rows are integrity failures, never a reason to return an older row.
    fn get(
        &self,
        actor: &Principal,
        case_id: CaseId,
        id: MeasureId,
    ) -> Result<MeasureRecordDetail, ApplicationError>;

    /// Bind case, identity, revision and capture digest without consulting fresh
    /// mutation eligibility. Older Valid, terminal and EnteredInError records
    /// remain readable. Authorized absence is NotFound; lost evidence is an
    /// integrity failure. Never replace the selected revision with a newer head.
    fn exact(
        &self,
        actor: &Principal,
        case_id: CaseId,
        reference: PrecautionaryMeasureRef,
    ) -> Result<MeasureRecordDetail, ApplicationError>;
}

pub trait MeasureRecordReadWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: MeasureRecordReadQuery,
    ) -> Result<MeasureRecordPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: MeasureId,
    ) -> Result<MeasureRecordDetail, ApplicationError>;
    fn exact(
        &self,
        token: &str,
        case_id: CaseId,
        reference: PrecautionaryMeasureRef,
    ) -> Result<MeasureRecordDetail, ApplicationError>;
}
