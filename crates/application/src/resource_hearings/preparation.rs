use super::*;
use crate::{
    cases::CaseActorSnapshot,
    identity::Principal,
    procedural_facts::{
        resolve_fact_participants, validate_fact_administration, FactSourceSelection,
    },
    procedural_resources::resource_receipt_matches,
    resource_activities::{verify_resource_sources, ResourceActivityError, ResourceCaptureRef},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    participants::DirectoryStatus,
    procedural_facts::{FactDeclaration, FactParticipantRef},
    procedural_resources::{ResourceId, ResourceStatus},
};

fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidInput(message.into())
}
fn mismatch() -> ApplicationError {
    ResourceActivityError::SourceMismatch.into()
}

pub fn prepare(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: ResourceHearingCommand,
    mut material: ResourceHearingMaterial,
) -> Result<ResourceHearingDraft, ApplicationError> {
    let head = &material.resource_head;
    if material.case_id != case
        || head.case_id != case
        || head.id != resource
        || command.resource.id != resource
    {
        return Err(mismatch());
    }
    resource_receipt_matches(hasher, head)?;
    validate_fact_administration(
        hasher,
        case,
        &material.administration,
        Some(&head.recorded_administration),
    )?;
    if head.revision != command.expected_resource_revision {
        return Err(ResourceActivityError::ResourceRevisionConflict.into());
    }
    if head.status != ResourceStatus::Active {
        return Err(ResourceActivityError::ResourceArchived.into());
    }
    if command.resource.revision > head.revision
        || command
            .act
            .is_some_and(|act| act.resource_revision > head.revision)
    {
        return Err(mismatch());
    }
    verify_resource_sources(
        hasher,
        case,
        resource,
        command.resource,
        command.act,
        &material.resource,
        material.act.as_ref(),
    )?;
    for source in std::iter::once(&material.resource).chain(material.act.iter()) {
        if source.recorded_at > head.recorded_at
            || (source.revision == head.revision && source != head)
        {
            return Err(mismatch());
        }
        validate_fact_administration(
            hasher,
            case,
            &material.administration,
            Some(&source.recorded_administration),
        )?;
    }
    for source in [&material.resource, head] {
        let FactDeclaration::Known(mode) = source.values.mode() else {
            return Err(invalid(
                "resource hearing requires an explicit written resource mode",
            ));
        };
        if !command
            .values
            .kind()
            .is_compatible_with(source.values.kind(), *mode)
        {
            return Err(invalid(
                "hearing kind is incompatible with the declared resource",
            ));
        }
    }
    let selected_support = command.values.scheduling_basis().support();
    let mut support = None;
    for captured in material.resource.sources.supports.iter().chain(
        material
            .act
            .iter()
            .flat_map(|row| row.act.iter().flat_map(|act| act.supports.iter())),
    ) {
        if captured.reference == selected_support.reference() {
            if captured.digest != selected_support.digest()
                || support
                    .as_ref()
                    .is_some_and(|previous| *previous != captured)
            {
                return Err(mismatch());
            }
            support = Some(captured);
        }
    }
    let support = support.cloned().ok_or_else(mismatch)?;
    if material.participants.len() != command.values.participants().len() {
        return Err(mismatch());
    }
    material
        .participants
        .sort_by_key(|person| person.id().as_uuid());
    let mut participants = Vec::with_capacity(material.participants.len());
    for (reference, source) in command
        .values
        .participants()
        .iter()
        .zip(&material.participants)
    {
        let selection = FactSourceSelection::select_participant(FactParticipantRef {
            id: reference.id(),
            revision: reference.revision(),
        });
        let mut resolved =
            resolve_fact_participants(hasher, case, &selection, std::slice::from_ref(source))?;
        let projection = resolved.pop().ok_or_else(mismatch)?;
        if projection.overview.directory_status != DirectoryStatus::Active {
            return Err(invalid("hearing participant is archived"));
        }
        participants.push(projection);
    }
    let mut draft = ResourceHearingDraft {
        case_id: case,
        command,
        resource: material.resource,
        act: material.act,
        support,
        participants,
        observed_administration: material.administration,
        observed_resource_head: ResourceCaptureRef {
            id: head.id,
            revision: head.revision,
            capture_digest: head.receipt.capture_digest,
        },
        recorded_by: CaseActorSnapshot {
            id: actor.id,
            email: actor.email.clone(),
        },
        submission_digest: Sha256Digest::from_array([0; 32]),
    };
    draft.submission_digest = super::canonical::digest(hasher, &draft)?;
    Ok(draft)
}
