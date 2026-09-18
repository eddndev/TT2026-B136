use super::{input_projection as input, metadata, result};
use crate::error::ApiError;
use application::deadlines::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
use serde_json::{json, Value};
pub(super) fn draft(
    v: DeadlineDraft,
    case: CaseId,
    expected: &DeadlineCommand,
) -> Result<Value, ApiError> {
    if v.case_id != case
        || input::command(&v.command)? != input::command(expected)?
        || v.result_revision != expected.result_revision()?
        || v.status != expected.status()
        || !metadata::definition_matches(&v.definition, expected)?
        || !metadata::attention_matches(&v.attention, expected)?
        || v.responsible.id != v.definition.responsible
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"case_id":case.to_string(),"actor_id":v.actor,"command":input::command(&v.command)?,"result_revision":v.result_revision.get(),"definition":input::definition(&v.definition)?,"calculation":metadata::calculation(&v.calculation,&v.definition,case)?,"responsible":metadata::responsible(&v.responsible)?,"attention":input::attention(&v.attention)?,"status":v.status.as_str(),"review_digest":v.review_digest.to_hex(),"capture_digest":v.capture_digest.to_hex(),"submission_digest":v.submission_digest.to_hex()}),
    )
}
pub(super) fn submitted(
    v: &DeadlineDetail,
    expected: &DeadlineCommand,
    digest: Sha256Digest,
) -> Result<(), ApiError> {
    metadata::shape(&v.receipt, v.revision, v.status, v.reason.as_ref())?;
    metadata::actor(&v.recorded_by)?;
    if v.tracking.is_some()
        || v.receipt.operation_id != expected.operation_id
        || v.receipt.action != expected.action()
        || v.receipt.expected_revision != expected.expected_revision()
        || v.receipt.submission_digest != digest
        || v.reason.as_ref() != expected.reason()
        || v.status != expected.status()
        || !metadata::definition_matches(&v.definition, expected)?
        || !metadata::attention_matches(&v.attention, expected)?
    {
        return Err(ApiError::internal());
    }
    Ok(())
}
pub(super) fn detail(
    v: DeadlineDetail,
    case: CaseId,
    id: DeadlineId,
    revision: Option<DeadlineRevision>,
) -> Result<Value, ApiError> {
    if v.id != id
        || v.case_id != case
        || revision.is_some_and(|r| r != v.revision)
        || v.responsible.id != v.definition.responsible
        || v.tracking.is_some()
        || v.receipt.action == DeadlineAction::Register && v.attention != DeadlineAttention::Pending
    {
        return Err(ApiError::internal());
    }
    metadata::shape(&v.receipt, v.revision, v.status, v.reason.as_ref())?;
    let recorded_by = metadata::actor(&v.recorded_by)?;
    Ok(
        json!({"id":v.id.to_string(),"case_id":case.to_string(),"revision":v.revision.get(),"definition":input::definition(&v.definition)?,"calculation":metadata::calculation(&v.calculation,&v.definition,case)?,"responsible":metadata::responsible(&v.responsible)?,"attention":input::attention(&v.attention)?,"status":v.status.as_str(),"reason":v.reason.as_ref().map(|r|r.as_str()),"receipt":metadata::receipt(&v.receipt),"recorded_at":result::instant(v.recorded_at),"recorded_by":recorded_by}),
    )
}
pub(super) use super::pages::{history, page};
