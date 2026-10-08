use super::PrecautionaryContext;
use crate::{identity::Principal, ApplicationError};
use domain::cases::CaseId;

/// Reauthorize the full current principal and case membership under the shared
/// audit lock. Read current administration and stage together, validate exact
/// historical sources, and commit access audit before disclosing the observation.
/// A closed case remains readable; this observation does not authorize a mutation.
pub trait PrecautionaryContextReadStore: Send + Sync {
    fn get(
        &self,
        actor: &Principal,
        case_id: CaseId,
    ) -> Result<PrecautionaryContext, ApplicationError>;
}

pub trait PrecautionaryContextReadWorkflow: Send + Sync {
    fn get(&self, token: &str, case_id: CaseId) -> Result<PrecautionaryContext, ApplicationError>;
}
