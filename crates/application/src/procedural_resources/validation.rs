use super::context::validate_context;
use super::*;
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::DocumentHasher, identity::UserId};

pub fn resource_command_from_detail(
    detail: &ResourceDetail,
) -> Result<ResourceCommand, ApplicationError> {
    let expected =
        || ResourceRevision::new(detail.receipt.expected_revision).map_err(ApplicationError::from);
    let reason = || {
        detail
            .reason
            .clone()
            .ok_or_else(|| inconsistent("resource correction or state change lacks reason"))
    };
    let change = match detail.receipt.action {
        ResourceAction::Register => ResourceChange::Register {
            values: detail.values.clone(),
        },
        ResourceAction::Correct => ResourceChange::Correct {
            expected_revision: expected()?,
            values: detail.values.clone(),
            reason: reason()?,
        },
        ResourceAction::Archive => ResourceChange::Archive {
            expected_revision: expected()?,
            reason: reason()?,
        },
        ResourceAction::Reactivate => ResourceChange::Reactivate {
            expected_revision: expected()?,
            reason: reason()?,
        },
        ResourceAction::RecordAct | ResourceAction::CorrectAct => {
            let act = detail
                .act
                .as_ref()
                .ok_or_else(|| inconsistent("resource act receipt lacks its capture"))?;
            if detail.receipt.action == ResourceAction::RecordAct {
                ResourceChange::RecordAct {
                    expected_revision: expected()?,
                    act_id: act.id,
                    values: act.values.clone(),
                }
            } else {
                let revision = act
                    .revision
                    .get()
                    .checked_sub(1)
                    .filter(|n| *n > 0)
                    .ok_or_else(|| inconsistent("corrected act has no previous revision"))?;
                ResourceChange::CorrectAct {
                    expected_revision: expected()?,
                    act_id: act.id,
                    expected_act_revision: ResourceActRevision::new(revision)?,
                    values: act.values.clone(),
                    reason: reason()?,
                }
            }
        }
    };
    Ok(ResourceCommand {
        operation_id: detail.receipt.operation_id,
        resource_id: detail.id,
        change,
    })
}
/// Verifies captured values, readable sources, command and commit receipt without
/// replacing historical references or asserting current source authorization.
pub fn resource_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &ResourceDetail,
) -> Result<(), ApplicationError> {
    verify_receipt(hasher, detail).map_err(|error| match error {
        ApplicationError::ProceduralResource(ProceduralResourceError::StoredInconsistent(_)) => {
            error
        }
        _ => inconsistent("stored resource receipt or captured evidence is invalid"),
    })
}
fn verify_receipt(
    hasher: &dyn DocumentHasher,
    detail: &ResourceDetail,
) -> Result<(), ApplicationError> {
    let command = resource_command_from_detail(detail)?;
    if command.result_revision()? != detail.revision
        || detail.receipt.expected_revision != detail.revision.get() - 1
        || detail.reason.as_ref() != command.reason()
        || detail.recorded_by.email.is_empty()
        || detail.recorded_by.email.chars().any(char::is_control)
        || detail.recorded_at.offset() != time::UtcOffset::UTC
        || !(1..=9999).contains(&detail.recorded_at.year())
        || detail
            .recorded_administration
            .snapshot()
            .is_some_and(|a| detail.recorded_at < a.changed_at)
        || detail
            .recorded_stage
            .entry()
            .is_some_and(|s| detail.recorded_at < s.recorded_at())
    {
        return Err(inconsistent(
            "resource revision, reason or actor metadata differs",
        ));
    }
    match (detail.receipt.previous, detail.revision.get()) {
        (None, 1) => {}
        (Some(previous), current) if previous.revision.get() == current - 1 => {}
        _ => {
            return Err(inconsistent(
                "resource previous receipt does not match its revision",
            ))
        }
    }
    let is_act = matches!(
        command.action(),
        ResourceAction::RecordAct | ResourceAction::CorrectAct
    );
    if is_act != detail.act.is_some()
        || (command.action() == ResourceAction::Archive)
            != (detail.status == ResourceStatus::Archived)
    {
        return Err(inconsistent(
            "resource status or act presence differs from action",
        ));
    }
    if let Some(act) = &detail.act {
        super::sources::validate_supports(&act.values.direct_supports(), &act.supports)?;
        match (command.action(), act.revision.get(), act.previous) {
            (ResourceAction::RecordAct, 1, None) => {}
            (ResourceAction::CorrectAct, n, Some(previous))
                if n > 1 && previous.revision < detail.revision => {}
            _ => {
                return Err(inconsistent(
                    "resource act revision or previous capture differs",
                ))
            }
        }
    }
    super::sources::validate_selection(detail.case_id, &detail.values, &detail.sources)?;
    validate_context(
        hasher,
        detail.case_id,
        &detail.recorded_administration,
        &detail.recorded_stage,
    )?;
    crate::procedural_facts::validate_fact_administration(
        hasher,
        detail.case_id,
        &detail.recorded_administration,
        None,
    )?;
    let draft = super::canonical::draft_from_detail(detail, command);
    if hasher.hash_bytes(&detail.values.canonical_bytes()) != detail.receipt.values_digest
        || hasher.hash_bytes(&resource_sources_bytes(&detail.sources)?)
            != detail.receipt.sources_digest
        || hasher.hash_bytes(&resource_submission_bytes(hasher, &draft)?)
            != detail.receipt.submission_digest
        || hasher.hash_bytes(&super::resource_capture_bytes(detail))
            != detail.receipt.capture_digest
    {
        return Err(inconsistent(
            "resource captured evidence or receipt digest differs",
        ));
    }
    Ok(())
}
pub(super) fn replay(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    case_id: CaseId,
    command: &ResourceCommand,
    detail: &ResourceDetail,
) -> Result<(), ApplicationError> {
    resource_receipt_matches(hasher, detail)?;
    if detail.case_id != case_id
        || detail.recorded_by.id != actor
        || resource_command_from_detail(detail)? != *command
    {
        return Err(ProceduralResourceError::OperationConflict.into());
    }
    Ok(())
}
