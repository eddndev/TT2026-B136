use super::*;
use crate::{
    cases::{case_administration_digest, CaseAdministrativeStatus},
    deadline_evaluations::{
        evaluate_profiled_deadline, DeadlineEvaluationError, DeadlineEvaluationRecord,
    },
    deadline_profiles::{deadline_profile_receipt_matches, DeadlineProfileStatus},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    identity::{Permission, UserId},
};

/// Pure validation is reusable inside the audited commit. The store must independently
/// resolve current identity, membership and responsible eligibility in that transaction.
pub fn prepare_deadline_change(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    case_id: CaseId,
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
) -> Result<PreparedDeadlineChange, ApplicationError> {
    command.result_revision()?;
    if preparation.case_id != case_id || preparation.deadline_id != command.deadline_id {
        return Err(inconsistent("prepared case or deadline differs"));
    }
    if let Some(admin) = preparation.administration.snapshot() {
        if admin.case_id != case_id
            || case_administration_digest(hasher, &admin.values) != admin.values_digest
        {
            return Err(inconsistent("prepared administration differs"));
        }
    }
    if preparation.administration.values().status() == CaseAdministrativeStatus::Closed {
        return Err(ApplicationError::CaseClosed);
    }
    if let Some(base) = &preparation.base {
        deadline_receipt_matches(hasher, base)?;
        if base.case_id != case_id || base.id != command.deadline_id {
            return Err(inconsistent("base belongs to another deadline or case"));
        }
    }
    let base = match &command.change {
        DeadlineChange::Register { .. } => {
            if preparation.base.is_some() {
                return Err(DeadlineError::RevisionConflict.into());
            }
            None
        }
        _ => {
            let base = preparation.base.as_ref().ok_or(DeadlineError::NotFound)?;
            if base.revision.get() != command.expected_revision() {
                return Err(DeadlineError::RevisionConflict.into());
            }
            if base.status == DeadlineStatus::Retired {
                return Err(DeadlineError::Retired.into());
            }
            Some(base)
        }
    };
    let attention = match &command.change {
        DeadlineChange::SetAttention { attention, .. } => attention.clone(),
        _ => base.map_or(DeadlineAttention::Pending, |b| b.attention.clone()),
    };
    let (definition, calculation, responsible) = match &command.change {
        DeadlineChange::Register { definition } | DeadlineChange::Correct { definition, .. } => {
            let (calculation, responsible) = evaluate(hasher, case_id, definition, &preparation)?;
            (definition.clone(), calculation, responsible)
        }
        _ => {
            if preparation.resolved.is_some() || preparation.responsible.is_some() {
                return Err(inconsistent(
                    "attention and retirement must preserve captured inputs",
                ));
            }
            let base = base.ok_or(DeadlineError::NotFound)?;
            (
                base.definition.clone(),
                base.calculation.clone(),
                base.responsible.clone(),
            )
        }
    };
    let content = canonical::Content {
        definition: &definition,
        calculation: &calculation,
        responsible: &responsible,
        attention: &attention,
        status: command.status(),
    };
    let review_digest = canonical::state_digest(hasher, content, false)?;
    let capture_digest = canonical::state_digest(hasher, content, true)?;
    let submission_digest = canonical::submission_digest(
        hasher,
        actor,
        case_id,
        command.deadline_id,
        &DeadlineReceipt {
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            review_digest,
            capture_digest,
            submission_digest: review_digest,
        },
        review_digest,
        command.reason(),
    );
    Ok(PreparedDeadlineChange {
        actor,
        case_id,
        command,
        preparation,
        definition,
        calculation,
        responsible,
        attention,
        review_digest,
        capture_digest,
        submission_digest,
    })
}

fn evaluate(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    definition: &DeadlineDefinition,
    preparation: &DeadlinePreparation,
) -> Result<(DeadlineCalculation, DeadlineResponsibleSnapshot), ApplicationError> {
    if definition.input.selection.case_id != case_id {
        return Err(DeadlineError::Invalid("input.case_id").into());
    }
    let resolved = preparation
        .resolved
        .as_ref()
        .ok_or_else(|| inconsistent("prepared inputs are missing"))?;
    let responsible = preparation
        .responsible
        .as_ref()
        .ok_or(DeadlineError::ResponsibleUnavailable)?;
    if responsible.id != definition.responsible
        || !responsible.role.allows(Permission::ReadDeadline)
        || responsible.email.trim().is_empty()
    {
        return Err(DeadlineError::ResponsibleUnavailable.into());
    }
    deadline_profile_receipt_matches(hasher, &resolved.profile)?;
    deadline_profile_receipt_matches(hasher, &resolved.profile_head)?;
    if resolved.profile.id != definition.profile.id
        || resolved.profile.revision != definition.profile.revision
    {
        return Err(inconsistent(
            "resolved profile differs from exact selection",
        ));
    }
    if resolved.profile != resolved.profile_head
        || resolved.profile.status != DeadlineProfileStatus::Published
    {
        return Err(DeadlineError::ProfileUnavailable.into());
    }
    if resolved.material.administration != preparation.administration {
        return Err(inconsistent(
            "resolved administration differs from current preparation",
        ));
    }
    let result = evaluate_profiled_deadline(
        hasher,
        &resolved.profile.definition,
        &definition.input,
        &resolved.material,
    )
    .map_err(|error| match error {
        DeadlineEvaluationError::Inputs(error) => error,
        DeadlineEvaluationError::Invalid(field) => {
            ApplicationError::Deadline(DeadlineError::Invalid(field))
        }
    })?;
    Ok((
        DeadlineCalculation {
            profile: resolved.profile.clone(),
            material: resolved.material.clone(),
            result: DeadlineEvaluationRecord::capture(&result),
        },
        responsible.clone(),
    ))
}
