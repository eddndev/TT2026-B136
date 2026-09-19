use super::*;
use crate::{
    case_stages::{case_stage_digest, CaseStageEntry},
    cases::case_administration_digest,
    procedural_facts::validate_fact_administration,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};

pub(super) fn validate_context(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    admin: &crate::cases::CurrentCaseAdministration,
    stage: &crate::case_stages::CurrentCaseStage,
) -> Result<(), ApplicationError> {
    if let Some(snapshot) = admin.snapshot() {
        if snapshot.case_id != case_id
            || case_administration_digest(hasher, &snapshot.values) != snapshot.values_digest
        {
            return Err(inconsistent(
                "resource administration scope or digest differs",
            ));
        }
    }
    if let Some(entry) = stage.entry() {
        if entry.case_id() != case_id {
            return Err(inconsistent("resource stage belongs to another case"));
        }
        match entry {
            CaseStageEntry::Initial(value)
                if value.stage_revision.get() != 1 || value.administration_revision.get() != 1 =>
            {
                return Err(inconsistent("resource initial stage counters differ"));
            }
            CaseStageEntry::Changed(value)
                if case_stage_digest(hasher, &value.values) != value.values_digest =>
            {
                return Err(inconsistent("resource stage digest differs"));
            }
            _ => {}
        }
        if let Some(snapshot) = admin.snapshot() {
            let (revision, digest) = match entry {
                CaseStageEntry::Initial(v) => (v.administration_revision, v.administration_digest),
                CaseStageEntry::Changed(v) => (v.administration_revision, v.administration_digest),
            };
            if revision > snapshot.revision
                || (revision == snapshot.revision && digest != snapshot.values_digest)
            {
                return Err(inconsistent(
                    "resource stage administration contradicts its observation",
                ));
            }
        } else {
            return Err(inconsistent(
                "registered resource stage lacks recorded administration",
            ));
        }
    }
    Ok(())
}
pub(super) fn material(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    command: &ResourceCommand,
    material: &ResourceMaterial,
) -> Result<(), ApplicationError> {
    command.result_revision()?;
    if material.case_id != case_id {
        return Err(inconsistent("resource preparation belongs to another case"));
    }
    validate_context(hasher, case_id, &material.administration, &material.stage)?;
    validate_fact_administration(
        hasher,
        case_id,
        &material.administration,
        material.base.as_ref().map(|v| &v.recorded_administration),
    )?;
    match (&material.base, command.expected_revision()) {
        (None, 0) => {}
        (Some(base), n) if n > 0 => {
            resource_receipt_matches(hasher, base)?;
            if base.case_id != case_id || base.id != command.resource_id {
                return Err(inconsistent("resource base scope differs"));
            }
            if base.revision.get() != n {
                return Err(ProceduralResourceError::RevisionConflict.into());
            }
            if base.receipt.operation_id == command.operation_id {
                return Err(ProceduralResourceError::OperationConflict.into());
            }
            match (base.status, command.action()) {
                (ResourceStatus::Archived, ResourceAction::Reactivate) => {}
                (ResourceStatus::Archived, _) => {
                    return Err(ProceduralResourceError::Archived.into())
                }
                (ResourceStatus::Active, ResourceAction::Reactivate) => {
                    return Err(ProceduralResourceError::StateUnchanged.into())
                }
                _ => {}
            }
            match (base.recorded_stage.entry(), material.stage.entry()) {
                (Some(_), None) => return Err(inconsistent("resource observed stage regressed")),
                (Some(old), Some(new))
                    if new.stage_revision() < old.stage_revision()
                        || (new.stage_revision() == old.stage_revision() && new != old) =>
                {
                    return Err(inconsistent(
                        "resource observed stage changed at the same revision",
                    ))
                }
                _ => {}
            }
        }
        _ => return Err(ProceduralResourceError::RevisionConflict.into()),
    }
    let changes_values = matches!(
        command.action(),
        ResourceAction::Register | ResourceAction::Correct
    );
    if !changes_values && (material.resolution.is_some() || !material.appellants.is_empty()) {
        return Err(inconsistent(
            "resource retained sources were replaced during an act or organizational command",
        ));
    }
    if !matches!(command.action(), ResourceAction::CorrectAct) && material.act_base.is_some() {
        return Err(inconsistent("unexpected prior resource act"));
    }
    Ok(())
}
