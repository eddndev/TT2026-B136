use super::ProceduralFactError;
use crate::{
    cases::{case_administration_digest, CurrentCaseAdministration},
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
};

/// Validates observed administration against an optional historical capture, without
/// authorization or an administrative CAS. The store must resolve an unrevised
/// baseline under case_id because its metadata contains no case identity.
pub fn validate_fact_administration(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    observed: &CurrentCaseAdministration,
    captured: Option<&CurrentCaseAdministration>,
) -> Result<(), ApplicationError> {
    validate_snapshot(hasher, case_id, observed)?;
    if let Some(captured) = captured {
        validate_snapshot(hasher, case_id, captured)?;
        match (observed, captured) {
            (
                CurrentCaseAdministration::Unrevised(current),
                CurrentCaseAdministration::Unrevised(previous),
            ) if current != previous => {
                return Err(inconsistent("unrevised administration metadata changed"));
            }
            (CurrentCaseAdministration::Unrevised(_), CurrentCaseAdministration::Recorded(_)) => {
                return Err(inconsistent("recorded administration became unrevised"));
            }
            (
                CurrentCaseAdministration::Recorded(current),
                CurrentCaseAdministration::Recorded(previous),
            ) => {
                if current.revision < previous.revision {
                    return Err(inconsistent("observed administration predates its capture"));
                }
                if current.revision == previous.revision && current != previous {
                    return Err(inconsistent("exact administration snapshot changed"));
                }
            }
            _ => {}
        }
    }
    if observed
        .snapshot()
        .is_some_and(|value| value.values.status() == CaseAdministrativeStatus::Closed)
    {
        return Err(ApplicationError::CaseClosed);
    }
    Ok(())
}

fn validate_snapshot(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    administration: &CurrentCaseAdministration,
) -> Result<(), ApplicationError> {
    if let Some(snapshot) = administration.snapshot() {
        if snapshot.case_id != case_id {
            return Err(inconsistent("administration belongs to another case"));
        }
        if case_administration_digest(hasher, &snapshot.values) != snapshot.values_digest {
            return Err(inconsistent(
                "administration digest differs from its values",
            ));
        }
    }
    Ok(())
}

fn inconsistent(message: &str) -> ApplicationError {
    ProceduralFactError::StoredInconsistent(message.into()).into()
}
