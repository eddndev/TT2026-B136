use super::{input_projection as input, receipt_projection, result, sources};
use crate::{deadline_profiles::definition_projection, error::ApiError};
use application::{
    deadline_profiles::{DeadlineProfileCollection, DeadlineProfileStatus},
    deadline_reevaluation::{encode_tracked_submission, TrackedAction, TrackedSubmission},
    deadlines::*,
};
use domain::{cases::CaseId, identity::Permission, procedural_facts::FactText};
use serde_json::{json, Value};
pub(super) fn actor(v: &DeadlineActorSnapshot) -> Result<Value, ApiError> {
    receipt_projection::author(v)
}
pub(super) fn responsible(v: &DeadlineResponsibleSnapshot) -> Result<Value, ApiError> {
    if v.email.trim().is_empty() || !v.role.allows(Permission::ReadDeadline) {
        return Err(ApiError::internal());
    }
    Ok(json!({"id":v.id,"email":v.email,"role":v.role.as_str()}))
}
pub(super) fn receipt(v: &DeadlineReceipt, case: CaseId) -> Result<Value, ApiError> {
    Ok(
        json!({"operation_id":v.operation_id.to_string(),"action":v.action.as_str(),"expected_revision":v.expected_revision,"review_digest":v.review_digest.to_hex(),"capture_digest":v.capture_digest.to_hex(),"submission_digest":v.submission_digest.to_hex(),"version":receipt_projection::version(&v.version,case)?}),
    )
}
pub(super) fn shape(
    v: &DeadlineReceipt,
    revision: DeadlineRevision,
    status: DeadlineStatus,
    reason: Option<&FactText>,
    author: &DeadlineActorSnapshot,
    case: CaseId,
) -> Result<(), ApiError> {
    receipt_projection::author(author)?;
    receipt_projection::version(&v.version, case)?;
    let valid = match v.action {
        DeadlineAction::Register => {
            v.expected_revision == 0 && reason.is_none() && status == DeadlineStatus::Active
        }
        DeadlineAction::Correct | DeadlineAction::SetAttention | DeadlineAction::Reevaluate => {
            v.expected_revision > 0 && reason.is_some() && status == DeadlineStatus::Active
        }
        DeadlineAction::Retire => {
            v.expected_revision > 0 && reason.is_some() && status == DeadlineStatus::Retired
        }
    };
    if !valid || v.expected_revision.checked_add(1) != Some(revision.get()) {
        return Err(ApiError::internal());
    }
    match &v.version {
        DeadlineReceiptVersion::Legacy => {
            if author.user_id().is_none() || v.action == DeadlineAction::Reevaluate {
                return Err(ApiError::internal());
            }
        }
        DeadlineReceiptVersion::Tracked(metadata) => {
            let action = match v.action {
                DeadlineAction::Register => TrackedAction::Register,
                DeadlineAction::Correct => TrackedAction::Correct,
                DeadlineAction::SetAttention => TrackedAction::SetAttention,
                DeadlineAction::Retire => TrackedAction::Retire,
                DeadlineAction::Reevaluate => TrackedAction::Reevaluate,
            };
            // The encoder checks shape; source identity and digests belong to the service.
            encode_tracked_submission(&TrackedSubmission {
                case_id: case,
                deadline_id: DeadlineId::from_uuid(uuid::Uuid::nil()),
                operation_id: v.operation_id,
                action,
                expected_revision: v.expected_revision,
                review_digest: v.review_digest,
                observations_digest: metadata.observations_digest,
                predecessor: metadata.predecessor,
                author: author.clone(),
                reason: reason.map(|r| r.as_str().to_owned()),
                cause: metadata.cause,
            })
            .map_err(|_| ApiError::internal())?;
        }
    }
    Ok(())
}
pub(super) fn calculation(
    v: &DeadlineCalculation,
    definition: &DeadlineDefinition,
    case: CaseId,
) -> Result<Value, ApiError> {
    let p = &v.profile;
    if p.id != definition.profile.id
        || p.revision != definition.profile.revision
        || p.status != DeadlineProfileStatus::Published
        || !DeadlineProfileCollection::ForCase(case).includes(p.definition.scope())
        || v.result.requirement() != p.definition.trigger()
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"profile":{"id":p.id.to_string(),"revision":p.revision.get(),"algorithm":p.algorithm.as_str(),"title":p.definition.title().as_str(),"scope":definition_projection::scope(p.definition.scope()),"status":p.status.as_str(),"definition_digest":p.definition_digest.to_hex(),"submission_digest":p.receipt.submission_digest.to_hex(),"href":format!("/api/v1/cases/{case}/deadline-profiles/{}/revisions/{}",p.id,p.revision.get())},"material":sources::material(&v.material,definition,case)?,"result":result::project(&v.result)?}),
    )
}
pub(super) fn definition_matches(
    definition: &DeadlineDefinition,
    command: &DeadlineCommand,
) -> Result<bool, ApiError> {
    match &command.change {
        DeadlineChange::Register {
            definition: expected,
        }
        | DeadlineChange::Correct {
            definition: expected,
            ..
        } => Ok(input::definition(definition)? == input::definition(expected)?),
        _ => Ok(true),
    }
}
pub(super) fn attention_matches(
    attention: &DeadlineAttention,
    command: &DeadlineCommand,
) -> Result<bool, ApiError> {
    match &command.change {
        DeadlineChange::Register { .. } => Ok(matches!(attention, DeadlineAttention::Pending)),
        DeadlineChange::SetAttention {
            attention: expected,
            ..
        } => Ok(input::attention(attention)? == input::attention(expected)?),
        _ => Ok(true),
    }
}
