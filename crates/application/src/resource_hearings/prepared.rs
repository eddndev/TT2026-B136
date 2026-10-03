use super::*;
use crate::{identity::Principal, ApplicationError};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher, procedural_resources::ResourceId,
    resource_hearings::ResourceHearingRevision,
};
use std::sync::Arc;

/// Cannot be constructed without validating the declared scheduling sources.
pub struct PreparedResourceHearing {
    draft: ResourceHearingDraft,
    material: ResourceHearingMaterial,
    actor: Principal,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl PreparedResourceHearing {
    pub fn draft(&self) -> &ResourceHearingDraft {
        &self.draft
    }
    pub fn material(&self) -> &ResourceHearingMaterial {
        &self.material
    }
    pub fn actor(&self) -> &Principal {
        &self.actor
    }
    pub fn into_creation(
        self,
        recorded_at: OffsetDateTime,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        let mut hearing = ResourceHearingDetail {
            review: self.draft,
            material: self.material,
            revision: ResourceHearingRevision::initial(),
            recorded_at,
            capture_digest: domain::crypto::Sha256Digest::from_array([0; 32]),
        };
        hearing.capture_digest = super::receipt::capture_digest(self.hasher.as_ref(), &hearing);
        let result = ResourceHearingCreation {
            origin: super::receipt::origin(&hearing),
            hearing,
        };
        resource_hearing_creation_matches(self.hasher.as_ref(), &result)?;
        Ok(result)
    }
}
pub fn prepare_resource_hearing_change(
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: ResourceHearingCommand,
    mut material: ResourceHearingMaterial,
) -> Result<PreparedResourceHearing, ApplicationError> {
    material.participants.sort_by_key(|p| p.id().as_uuid());
    let draft = super::preparation::prepare(
        hasher.as_ref(),
        actor,
        case,
        resource,
        command,
        material.clone(),
    )?;
    Ok(PreparedResourceHearing {
        draft,
        material,
        actor: actor.clone(),
        hasher,
    })
}
