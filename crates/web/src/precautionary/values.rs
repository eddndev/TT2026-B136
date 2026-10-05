use super::primitives::*;
use crate::{error::ApiError, procedural_facts::object::Object};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearings::{HearingNote, HearingParticipantRef, HearingSupportRef, HearingVenue},
    participants::{ParticipantId, ParticipantRevision},
    precautionary_hearings::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Support {
    document_id: String,
    version: u32,
    digest: String,
}
impl Support {
    fn validate(self) -> Result<HearingSupportRef, ApiError> {
        Ok(HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(uuid(&self.document_id)?),
                version: DocumentVersion::new(self.version).map_err(|_| invalid())?,
            },
            digest(&self.digest)?,
        ))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Basis {
    statement: String,
    support: Object<Support>,
    locator: String,
}
impl Basis {
    fn validate(self) -> Result<PrecautionaryHearingSchedulingBasis, ApiError> {
        Ok(PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new(&self.statement).map_err(|_| invalid())?,
            self.support.0.validate()?,
            HearingNote::new(&self.locator).map_err(|_| invalid())?,
        ))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Participant {
    participant_id: String,
    revision: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    id: String,
    revision: u32,
    capture_digest: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Values {
    purpose: String,
    scheduled_at: String,
    modality: String,
    venue: String,
    #[serde(deserialize_with = "nullable")]
    note: Option<String>,
    #[serde(deserialize_with = "selected")]
    participants: Vec<Object<Participant>>,
    scheduling_basis: Object<Basis>,
    #[serde(deserialize_with = "selected")]
    review_targets: Vec<Object<Target>>,
}
impl Values {
    pub(super) fn validate(self) -> Result<PrecautionaryHearingValues, ApiError> {
        PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
            purpose: self.purpose.parse().map_err(|_| invalid())?,
            scheduled_at: scheduled(&self.scheduled_at)?,
            modality: self.modality.parse().map_err(|_| invalid())?,
            venue: HearingVenue::new(&self.venue).map_err(|_| invalid())?,
            note: HearingNote::optional(self.note.as_deref()).map_err(|_| invalid())?,
            participants: self
                .participants
                .into_iter()
                .map(|p| {
                    Ok(HearingParticipantRef::new(
                        ParticipantId::from_uuid(uuid(&p.0.participant_id)?),
                        ParticipantRevision::new(p.0.revision).map_err(|_| invalid())?,
                    ))
                })
                .collect::<Result<_, ApiError>>()?,
            scheduling_basis: self.scheduling_basis.0.validate()?,
            review_targets: self
                .review_targets
                .into_iter()
                .map(|r| {
                    Ok(PrecautionaryMeasureRef::new(
                        MeasureId::from_uuid(uuid(&r.0.id)?),
                        MeasureRevision::new(r.0.revision).map_err(|_| invalid())?,
                        digest(&r.0.capture_digest)?,
                    ))
                })
                .collect::<Result<_, ApiError>>()?,
        })
        .map_err(|_| invalid())
    }
}
