use super::*;
use crate::{
    identity::Principal, procedural_facts::validate_fact_administration, ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher};

pub fn resource_activity_command_from_detail(
    detail: &ResourceActivityDetail,
) -> Result<ResourceActivityCommand, ApplicationError> {
    let change = match detail.receipt.action {
        ResourceActivityAction::Link
            if detail.receipt.expected_revision == 0 && detail.reason.is_none() =>
        {
            ResourceActivityChange::Link {
                selection: detail.selection,
            }
        }
        ResourceActivityAction::Unlink => ResourceActivityChange::Unlink {
            expected_revision: ResourceActivityRevision::new(detail.receipt.expected_revision)
                .map_err(|_| inconsistent("unlink predecessor is not positive"))?,
            reason: detail
                .reason
                .clone()
                .ok_or_else(|| inconsistent("unlink reason is absent"))?,
        },
        _ => return Err(inconsistent("association receipt action or reason differs")),
    };
    Ok(ResourceActivityCommand {
        operation_id: detail.receipt.operation_id,
        association_id: detail.id,
        expected_resource_revision: detail.receipt.expected_resource_revision,
        change,
    })
}
pub fn resource_activity_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &ResourceActivityDetail,
) -> Result<(), ApplicationError> {
    verify(hasher, detail).map_err(|error| match error {
        ApplicationError::ResourceActivity(ResourceActivityError::StoredInconsistent(_)) => error,
        _ => inconsistent("stored association or source receipt is invalid"),
    })
}
fn verify(
    hasher: &dyn DocumentHasher,
    detail: &ResourceActivityDetail,
) -> Result<(), ApplicationError> {
    let command = resource_activity_command_from_detail(detail)?;
    let expected_status = match command.action() {
        ResourceActivityAction::Link => ResourceActivityStatus::Linked,
        ResourceActivityAction::Unlink => ResourceActivityStatus::Unlinked,
    };
    if command.result_revision()? != detail.revision
        || detail.status != expected_status
        || detail.recorded_resource_head.id != detail.resource_id
        || detail.recorded_resource_head.revision != command.expected_resource_revision
        || detail.selection.resource.id != detail.resource_id
        || detail.selection.resource.revision > detail.recorded_resource_head.revision
        || detail
            .selection
            .act
            .is_some_and(|v| v.resource_revision > detail.recorded_resource_head.revision)
        || detail.recorded_by.email.is_empty()
        || detail.recorded_by.email.trim() != detail.recorded_by.email
        || detail.recorded_by.email.chars().any(char::is_control)
    {
        return Err(inconsistent(
            "association identity, actor, status or expected head differs",
        ));
    }
    match (detail.receipt.previous, detail.revision.get()) {
        (None, 1) => {}
        (Some(previous), n) if previous.revision.get() == n - 1 => {}
        _ => return Err(inconsistent("association predecessor differs")),
    }
    validate_fact_administration(
        hasher,
        detail.case_id,
        &detail.recorded_administration,
        None,
    )?;
    super::sources::verify(
        hasher,
        detail.case_id,
        detail.resource_id,
        detail.selection,
        &detail.sources,
    )?;
    valid_time(detail.recorded_at)?;
    if detail.recorded_at < super::sources::latest_time(&detail.sources)
        || detail
            .recorded_administration
            .snapshot()
            .is_some_and(|a| detail.recorded_at < a.changed_at)
    {
        return Err(inconsistent("association clock predates its evidence"));
    }
    let draft = super::canonical::draft_from_detail(detail)?;
    if hasher.hash_bytes(&resource_activity_submission_bytes(hasher, &draft)?)
        != detail.receipt.submission_digest
        || hasher.hash_bytes(&resource_activity_capture_bytes(detail))
            != detail.receipt.capture_digest
    {
        return Err(inconsistent(
            "association receipt differs from captured evidence",
        ));
    }
    Ok(())
}
pub(super) fn valid_time(value: OffsetDateTime) -> Result<(), ApplicationError> {
    if value.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&value.year()) {
        return Err(inconsistent(
            "association timestamp must be representable UTC",
        ));
    }
    Ok(())
}
pub(super) fn replay(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceActivityCommand,
    detail: &ResourceActivityDetail,
) -> Result<(), ApplicationError> {
    resource_activity_receipt_matches(hasher, detail)?;
    if detail.case_id != case
        || detail.resource_id != resource
        || detail.recorded_by.id != actor.id
        || resource_activity_command_from_detail(detail)? != *command
    {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    Ok(())
}
