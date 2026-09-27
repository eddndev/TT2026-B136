use super::inconsistent;
use super::*;
use crate::{
    cases::{CaseActorSnapshot, CaseAdministrativeStatus},
    deadlines::*,
    identity::Principal,
    procedural_facts::validate_fact_administration,
    procedural_resources::{resource_receipt_matches, ResourceStatus},
    resource_activities::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
};
use std::sync::Arc;

pub(super) fn register(command: &ResourceDeadlineCommand) -> Result<(), ApplicationError> {
    if !matches!(
        command.deadline.clone().into_parts().0.change,
        DeadlineChange::Register { .. }
    ) {
        return Err(DeadlineError::Invalid("contextual creation requires register").into());
    }
    Ok(())
}
pub(super) fn association_command(
    command: &ResourceDeadlineCommand,
    target: ResourceActivityTarget,
) -> ResourceActivityCommand {
    ResourceActivityCommand {
        operation_id: ResourceActivityOperationId::from_uuid(
            command
                .deadline
                .clone()
                .into_parts()
                .0
                .operation_id
                .as_uuid(),
        ),
        association_id: command.association_id,
        expected_resource_revision: command.expected_resource_revision,
        change: ResourceActivityChange::Link {
            selection: ResourceActivitySelection {
                resource: command.resource,
                act: command.act,
                target,
            },
        },
    }
}
/// Validate the prospective capture without inventing a persisted timestamp.
pub fn prepare_resource_deadline_change(
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: ResourceDeadlineCommand,
    material: ResourceDeadlineMaterial,
) -> Result<PreparedResourceDeadline, ApplicationError> {
    register(&command)?;
    if material.case_id != case
        || material.resource_head.case_id != case
        || material.resource_head.id != resource
        || command.resource.id != resource
        || material.deadline.administration != material.administration
    {
        return Err(inconsistent(
            "contextual preparation scope or administration differs",
        ));
    }
    if material.administration.values().status() != CaseAdministrativeStatus::Active {
        return Err(ApplicationError::CaseClosed);
    }
    resource_receipt_matches(hasher.as_ref(), &material.resource_head)?;
    validate_fact_administration(
        hasher.as_ref(),
        case,
        &material.administration,
        Some(&material.resource_head.recorded_administration),
    )?;
    if material.resource_head.revision != command.expected_resource_revision {
        return Err(ResourceActivityError::ResourceRevisionConflict.into());
    }
    if material.resource_head.status != ResourceStatus::Active {
        return Err(ResourceActivityError::ResourceArchived.into());
    }
    if command.resource.revision > material.resource_head.revision
        || command
            .act
            .is_some_and(|act| act.resource_revision > material.resource_head.revision)
    {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    crate::resource_activities::verify_resource_sources(
        hasher.as_ref(),
        case,
        resource,
        command.resource,
        command.act,
        &material.resource,
        material.act.as_ref(),
    )?;
    let (deadline_command, policies) = command.deadline.clone().into_parts();
    let parent = material
        .deadline
        .resolved
        .as_ref()
        .and_then(|resolved| resolved.notification_parent_head.clone());
    let deadline = prepare_tracked_deadline_change(
        hasher.as_ref(),
        DeadlineActorSnapshot::User {
            id: actor.id,
            email: actor.email.clone(),
        },
        case,
        deadline_command,
        material.deadline.clone(),
        policies,
        parent.as_ref(),
    )?;
    let deadline_draft = crate::deadlines::prepared_draft(&deadline)?;
    let association_command = association_command(
        &command,
        ResourceActivityTarget::Deadline {
            id: deadline.command().deadline_id,
            revision: DeadlineRevision::initial(),
            capture_digest: deadline.capture_digest(),
        },
    );
    let mut association = ResourceDeadlineAssociationDraft {
        command: association_command,
        resource: material.resource.clone(),
        act: material.act.clone(),
        recorded_by: CaseActorSnapshot {
            id: actor.id,
            email: actor.email.clone(),
        },
        observed_administration: material.administration.clone(),
        observed_resource_head: ResourceCaptureRef {
            id: resource,
            revision: material.resource_head.revision,
            capture_digest: material.resource_head.receipt.capture_digest,
        },
        submission_digest: Sha256Digest::from_array([0; 32]),
    };
    association.submission_digest =
        super::receipt::association_digest(hasher.as_ref(), case, resource, &association)?;
    let submission_digest = resource_deadline_submission_digest(
        hasher.as_ref(),
        deadline.submission_digest(),
        association.submission_digest,
    );
    let draft = ResourceDeadlineDraft {
        command,
        deadline: deadline_draft,
        association,
        submission_digest,
    };
    Ok(PreparedResourceDeadline {
        draft,
        material,
        deadline,
        hasher,
        actor: actor.clone(),
    })
}
