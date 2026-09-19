use super::*;
use crate::{
    cases::CaseActorSnapshot,
    identity::Principal,
    procedural_facts::validate_fact_administration,
    procedural_resources::{resource_receipt_matches, ResourceStatus},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
};
use std::sync::Arc;

pub(super) fn prepare(
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: ResourceActivityCommand,
    material: ResourceActivityMaterial,
) -> Result<PreparedResourceActivityChange, ApplicationError> {
    if material.case_id != case
        || material.resource_head.case_id != case
        || material.resource_head.id != resource
    {
        return Err(inconsistent("association preparation scope differs"));
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
    let (selection, status, previous) = match (&command.change, &material.base) {
        (ResourceActivityChange::Link { selection }, None) => {
            if material.resource_head.status != ResourceStatus::Active {
                return Err(ResourceActivityError::ResourceArchived.into());
            }
            (*selection, ResourceActivityStatus::Linked, None)
        }
        (
            ResourceActivityChange::Unlink {
                expected_revision, ..
            },
            Some(base),
        ) => {
            resource_activity_receipt_matches(hasher.as_ref(), base)?;
            if base.case_id != case
                || base.resource_id != resource
                || base.id != command.association_id
            {
                return Err(inconsistent("association base scope differs"));
            }
            if base.revision != *expected_revision {
                return Err(ResourceActivityError::RevisionConflict.into());
            }
            if base.status != ResourceActivityStatus::Linked {
                return Err(ResourceActivityError::StateUnchanged.into());
            }
            if base.receipt.operation_id == command.operation_id {
                return Err(ResourceActivityError::OperationConflict.into());
            }
            if base.sources != material.sources {
                return Err(ResourceActivityError::SourceMismatch.into());
            }
            validate_fact_administration(
                hasher.as_ref(),
                case,
                &material.administration,
                Some(&base.recorded_administration),
            )?;
            (
                base.selection,
                ResourceActivityStatus::Unlinked,
                Some(ResourceActivityRevisionRef {
                    revision: base.revision,
                    capture_digest: base.receipt.capture_digest,
                }),
            )
        }
        _ => return Err(ResourceActivityError::RevisionConflict.into()),
    };
    if selection.resource.revision > material.resource_head.revision
        || selection
            .act
            .is_some_and(|v| v.resource_revision > material.resource_head.revision)
    {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    super::sources::verify(
        hasher.as_ref(),
        case,
        resource,
        selection,
        &material.sources,
    )?;
    let mut draft = ResourceActivityDraft {
        case_id: case,
        resource_id: resource,
        result_revision: command.result_revision()?,
        command,
        selection,
        status,
        sources: material.sources.clone(),
        previous,
        recorded_by: CaseActorSnapshot {
            id: actor.id,
            email: actor.email.clone(),
        },
        observed_administration: material.administration.clone(),
        observed_resource_head: super::sources::resource_ref(&material.resource_head),
        submission_digest: Sha256Digest::from_array([0; 32]),
    };
    draft.submission_digest = hasher.hash_bytes(&resource_activity_submission_bytes(
        hasher.as_ref(),
        &draft,
    )?);
    Ok(PreparedResourceActivityChange {
        material,
        draft,
        hasher,
    })
}
