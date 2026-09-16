use super::{ProceduralFactCommand, ProceduralFactError, ProceduralFactSnapshot};
use crate::ApplicationError;
use domain::cases::CaseId;

/// Pure target and state guard; it does not validate canonical data, permissions, or sources.
/// The service composes those checks, and persistence must recheck the locked base.
pub fn validate_fact_base(
    case_id: CaseId,
    command: &ProceduralFactCommand,
    base: Option<&ProceduralFactSnapshot>,
) -> Result<(), ApplicationError> {
    command.result_revision()?;
    if let Some(snapshot) = base {
        if snapshot.case_id() != case_id {
            return Err(ProceduralFactError::StoredInconsistent(
                "procedural fact base belongs to another case".into(),
            )
            .into());
        }
        if snapshot.target() != command.target() {
            return Err(ProceduralFactError::StoredInconsistent(
                "procedural fact base target differs from the requested target".into(),
            )
            .into());
        }
    }
    command.validate_base(base.map(|snapshot| {
        let metadata = snapshot.metadata();
        (metadata.revision, metadata.status)
    }))?;
    Ok(())
}
