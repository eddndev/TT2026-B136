use super::{
    request::{parse_digest, parse_uuid},
    time::parse_time,
};
use crate::error::ApiError;
use application::ApplicationError;
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearings::*,
    participants::{ParticipantId, ParticipantRevision},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Support {
    pub document_id: String,
    pub version: u32,
    pub digest: String,
}
impl Support {
    pub fn validate(self) -> Result<HearingSupportRef, ApiError> {
        Ok(HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(parse_uuid(&self.document_id, "invalid_document_id")?),
                version: DocumentVersion::new(self.version)
                    .map_err(|_| ApiError::invalid_document_version())?,
            },
            parse_digest(&self.digest)?,
        ))
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Basis {
    pub statement: String,
    #[serde(deserialize_with = "super::object::deserialize")]
    pub support: Support,
}
impl Basis {
    fn validate(self) -> Result<HearingConvictionBasis, ApiError> {
        Ok(HearingConvictionBasis::new(
            HearingNote::new(&self.statement).map_err(ApplicationError::from)?,
            self.support.validate()?,
        ))
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Participant {
    pub participant_id: String,
    pub revision: u32,
}
impl Participant {
    fn validate(self) -> Result<HearingParticipantRef, ApiError> {
        Ok(HearingParticipantRef::new(
            ParticipantId::from_uuid(parse_uuid(&self.participant_id, "invalid_participant_id")?),
            ParticipantRevision::new(self.revision).map_err(ApplicationError::from)?,
        ))
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Values {
    pub kind: String,
    pub scheduled_at: String,
    pub modality: String,
    pub venue: String,
    pub note: Option<String>,
    #[serde(deserialize_with = "super::object::array")]
    pub participants: Vec<Participant>,
    #[serde(default, deserialize_with = "super::object::optional")]
    pub conviction_basis: Option<Basis>,
}
impl Values {
    pub fn validate(self) -> Result<HearingValues, ApiError> {
        HearingValues::new(HearingValuesInput {
            kind: self.kind.parse().map_err(ApplicationError::from)?,
            scheduled_at: parse_time(&self.scheduled_at)?,
            modality: self.modality.parse().map_err(ApplicationError::from)?,
            venue: HearingVenue::new(&self.venue).map_err(ApplicationError::from)?,
            note: HearingNote::optional(self.note.as_deref()).map_err(ApplicationError::from)?,
            participants: self
                .participants
                .into_iter()
                .map(Participant::validate)
                .collect::<Result<_, _>>()?,
            conviction_basis: self.conviction_basis.map(Basis::validate).transpose()?,
        })
        .map_err(|e| ApplicationError::from(e).into())
    }
}
