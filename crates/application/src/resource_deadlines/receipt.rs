use super::inconsistent;
use super::*;
use crate::{deadlines::*, identity::Principal, resource_activities::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
};
/// Versioned composition preserves each existing receipt's independent framing.
pub fn resource_deadline_submission_digest(
    hasher: &dyn DocumentHasher,
    deadline: Sha256Digest,
    association: Sha256Digest,
) -> Sha256Digest {
    let mut bytes = b"RDLTX1".to_vec();
    bytes.extend_from_slice(deadline.as_bytes());
    bytes.extend_from_slice(association.as_bytes());
    hasher.hash_bytes(&bytes)
}
pub(super) fn association_digest(
    hasher: &dyn DocumentHasher,
    case: CaseId,
    resource: ResourceId,
    draft: &ResourceDeadlineAssociationDraft,
) -> Result<Sha256Digest, ApplicationError> {
    let ResourceActivityChange::Link { selection } = draft.command.change else {
        return Err(inconsistent("contextual association is not a link"));
    };
    let intent = crate::resource_activities::ActivitySubmission {
        case_id: case,
        resource_id: resource,
        command: &draft.command,
        result_revision: ResourceActivityRevision::initial(),
        selection,
        status: ResourceActivityStatus::Linked,
        previous: None,
        recorded_by: &draft.recorded_by,
        observed_administration: &draft.observed_administration,
        observed_resource_head: draft.observed_resource_head,
    };
    Ok(
        hasher.hash_bytes(&crate::resource_activities::activity_submission_bytes(
            hasher, &intent,
        )?),
    )
}
/// Validate both immutable receipts and reconstruct the originally reviewed draft.
pub fn resource_deadline_result_draft(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceDeadlineCommand,
    result: &ResourceDeadlineResult,
) -> Result<ResourceDeadlineDraft, ApplicationError> {
    super::preparation::register(command)?;
    let d = &result.deadline;
    let a = &result.association;
    deadline_receipt_matches(hasher, d)?;
    resource_activity_receipt_matches(hasher, a)?;
    let (expected, policies) = command.deadline.clone().into_parts();
    let DeadlineChange::Register { definition } = &expected.change else {
        unreachable!()
    };
    let tracking = d
        .tracking
        .as_ref()
        .ok_or_else(|| inconsistent("contextual deadline has no tracking capture"))?;
    let link = super::preparation::association_command(
        command,
        ResourceActivityTarget::Deadline {
            id: d.id,
            revision: d.revision,
            capture_digest: d.receipt.capture_digest,
        },
    );
    if d.case_id != case
        || a.case_id != case
        || a.resource_id != resource
        || d.id != expected.deadline_id
        || d.revision != DeadlineRevision::initial()
        || d.receipt.action != DeadlineAction::Register
        || d.receipt.operation_id != expected.operation_id
        || d.recorded_by.user_id() != Some(actor.id)
        || a.recorded_by.id != actor.id
        || d.recorded_by
            != (DeadlineActorSnapshot::User {
                id: a.recorded_by.id,
                email: a.recorded_by.email.clone(),
            })
        || &d.definition != definition
        || Some(tracking.policies) != policies
        || resource_activity_command_from_detail(a)? != link
        || a.sources.target != ResourceActivityTargetDetail::Deadline(Box::new(d.clone()))
        || a.recorded_administration != tracking.administration
        || a.recorded_administration != d.calculation.material.administration
        || a.recorded_at != d.recorded_at
    {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    let digest = resource_deadline_submission_digest(
        hasher,
        d.receipt.submission_digest,
        a.receipt.submission_digest,
    );
    if digest != result.submission_digest {
        return Err(inconsistent(
            "contextual submission digest differs from its receipts",
        ));
    }
    Ok(ResourceDeadlineDraft {
        command: command.clone(),
        submission_digest: digest,
        deadline: DeadlineDraft {
            case_id: case,
            actor: actor.id,
            author: d.recorded_by.clone(),
            tracking: tracking.clone(),
            receipt_version: d.receipt.version.clone(),
            command: expected,
            result_revision: d.revision,
            definition: d.definition.clone(),
            calculation: d.calculation.clone(),
            responsible: d.responsible.clone(),
            attention: d.attention.clone(),
            status: d.status,
            review_digest: d.receipt.review_digest,
            capture_digest: d.receipt.capture_digest,
            submission_digest: d.receipt.submission_digest,
        },
        association: ResourceDeadlineAssociationDraft {
            command: link,
            resource: a.sources.resource.clone(),
            act: a.sources.act.clone(),
            recorded_by: a.recorded_by.clone(),
            observed_administration: a.recorded_administration.clone(),
            observed_resource_head: a.recorded_resource_head,
            submission_digest: a.receipt.submission_digest,
        },
    })
}
