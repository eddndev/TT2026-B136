use super::{input_projection as input, metadata, receipt_projection, result, tracking_projection};
use crate::error::ApiError;
use application::{deadline_currentness::DeadlineCurrent, deadlines::*};
use domain::{cases::CaseId, crypto::Sha256Digest};
use serde_json::{json, Value};
pub(super) fn draft(
    v: DeadlineDraft,
    case: CaseId,
    expected: &DeadlineHumanCommand,
) -> Result<Value, ApiError> {
    let (expected, policies) = expected.clone().into_parts();
    if v.case_id != case
        || input::command(&v.command)? != input::command(&expected)?
        || v.result_revision != expected.result_revision()?
        || v.status != expected.status()
        || !metadata::definition_matches(&v.definition, &expected)?
        || !metadata::attention_matches(&v.attention, &expected)?
        || v.responsible.id != v.definition.responsible
        || v.author.user_id() != Some(v.actor)
        || !matches!(v.receipt_version, DeadlineReceiptVersion::Tracked(_))
        || policies.is_some_and(|p| p != v.tracking.policies)
    {
        return Err(ApiError::internal());
    }
    let receipt = DeadlineReceipt {
        version: v.receipt_version,
        operation_id: v.command.operation_id,
        action: v.command.action(),
        expected_revision: v.command.expected_revision(),
        review_digest: v.review_digest,
        capture_digest: v.capture_digest,
        submission_digest: v.submission_digest,
    };
    metadata::shape(
        &receipt,
        v.result_revision,
        v.status,
        v.command.reason(),
        &v.author,
        case,
    )?;
    let tracking =
        super::tracking_context::project(&receipt, Some(&v.tracking), &v.calculation, case)?;
    let mut command = input::command(&v.command)?;
    if let Some(policies) = policies {
        command["change"]["tracking"] = tracking_projection::policies(&policies);
    }
    Ok(json!({
        "case_id":case.to_string(), "actor_id":v.actor,
        "author":receipt_projection::author(&v.author)?, "command":command,
        "result_revision":v.result_revision.get(), "definition":input::definition(&v.definition)?,
        "calculation":metadata::calculation(&v.calculation,&v.definition,case)?,
        "responsible":metadata::responsible(&v.responsible)?, "attention":input::attention(&v.attention)?,
        "status":v.status.as_str(), "tracking":tracking,
        "receipt_version":receipt_projection::version(&receipt.version,case)?,
        "review_digest":v.review_digest.to_hex(), "capture_digest":v.capture_digest.to_hex(),
        "submission_digest":v.submission_digest.to_hex()
    }))
}
pub(super) fn submitted(
    v: &DeadlineDetail,
    expected: &DeadlineHumanCommand,
    digest: Sha256Digest,
) -> Result<(), ApiError> {
    let (expected, policies) = expected.clone().into_parts();
    metadata::shape(
        &v.receipt,
        v.revision,
        v.status,
        v.reason.as_ref(),
        &v.recorded_by,
        v.case_id,
    )?;
    super::tracking_context::project(&v.receipt, v.tracking.as_ref(), &v.calculation, v.case_id)?;
    if !matches!(v.receipt.version, DeadlineReceiptVersion::Tracked(_))
        || v.recorded_by.user_id().is_none()
        || policies.is_some_and(|p| v.tracking.as_ref().is_none_or(|t| t.policies != p))
        || v.receipt.operation_id != expected.operation_id
        || v.receipt.action != expected.action()
        || v.receipt.expected_revision != expected.expected_revision()
        || v.receipt.submission_digest != digest
        || v.reason.as_ref() != expected.reason()
        || v.status != expected.status()
        || !metadata::definition_matches(&v.definition, &expected)?
        || !metadata::attention_matches(&v.attention, &expected)?
    {
        return Err(ApiError::internal());
    }
    Ok(())
}
pub(crate) fn detail(
    v: DeadlineDetail,
    case: CaseId,
    id: DeadlineId,
    revision: Option<DeadlineRevision>,
) -> Result<Value, ApiError> {
    if v.id != id
        || v.case_id != case
        || revision.is_some_and(|r| r != v.revision)
        || v.responsible.id != v.definition.responsible
        || v.receipt.action == DeadlineAction::Register && v.attention != DeadlineAttention::Pending
    {
        return Err(ApiError::internal());
    }
    metadata::shape(
        &v.receipt,
        v.revision,
        v.status,
        v.reason.as_ref(),
        &v.recorded_by,
        v.case_id,
    )?;
    let tracking =
        super::tracking_context::project(&v.receipt, v.tracking.as_ref(), &v.calculation, case)?;
    Ok(json!({
        "id":v.id.to_string(), "case_id":case.to_string(), "revision":v.revision.get(),
        "definition":input::definition(&v.definition)?,
        "calculation":metadata::calculation(&v.calculation,&v.definition,case)?,
        "responsible":metadata::responsible(&v.responsible)?, "attention":input::attention(&v.attention)?,
        "status":v.status.as_str(), "reason":v.reason.as_ref().map(|r|r.as_str()),
        "receipt":metadata::receipt(&v.receipt,case)?, "recorded_at":result::instant(v.recorded_at),
        "recorded_by":metadata::actor(&v.recorded_by)?, "tracking":tracking,
        "operational":{"freshness":"not_checked","checked_at":null,"changed_dependencies":[],"due_at":null}
    }))
}
pub(crate) fn current(
    value: DeadlineCurrent,
    case: CaseId,
    id: DeadlineId,
) -> Result<Value, ApiError> {
    let (capture, operational) = value.into_parts();
    if !operational.matches_capture(&capture) {
        return Err(ApiError::internal());
    }
    let mut output = detail(capture, case, id, None)?;
    output["operational"] = super::operational_projection::project(&operational);
    Ok(output)
}

pub(super) use super::pages::{history, page};
