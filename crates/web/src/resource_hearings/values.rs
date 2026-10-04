use super::request::{digest, invalid, uuid};
use crate::{error::ApiError, procedural_facts::object::Object};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearings::{HearingNote, HearingParticipantRef, HearingSupportRef, HearingTime, HearingVenue},
    participants::{ParticipantId, ParticipantRevision},
    resource_hearings::{
        ResourceHearingSchedulingBasis, ResourceHearingValues, ResourceHearingValuesInput,
    },
};
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

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
}

impl Basis {
    fn validate(self) -> Result<ResourceHearingSchedulingBasis, ApiError> {
        Ok(ResourceHearingSchedulingBasis::new(
            HearingNote::new(&self.statement).map_err(|_| invalid())?,
            self.support.0.validate()?,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Participant {
    participant_id: String,
    revision: u32,
}

impl Participant {
    fn validate(self) -> Result<HearingParticipantRef, ApiError> {
        Ok(HearingParticipantRef::new(
            ParticipantId::from_uuid(uuid(&self.participant_id)?),
            ParticipantRevision::new(self.revision).map_err(|_| invalid())?,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Values {
    kind: String,
    scheduled_at: String,
    modality: String,
    venue: String,
    note: Option<String>,
    participants: Vec<Object<Participant>>,
    scheduling_basis: Object<Basis>,
}

impl Values {
    pub(super) fn validate(self) -> Result<ResourceHearingValues, ApiError> {
        ResourceHearingValues::new(ResourceHearingValuesInput {
            kind: self.kind.parse().map_err(|_| invalid())?,
            scheduled_at: parse_time(&self.scheduled_at)?,
            modality: self.modality.parse().map_err(|_| invalid())?,
            venue: HearingVenue::new(&self.venue).map_err(|_| invalid())?,
            note: HearingNote::optional(self.note.as_deref()).map_err(|_| invalid())?,
            participants: self
                .participants
                .into_iter()
                .map(|participant| participant.0.validate())
                .collect::<Result<_, _>>()?,
            scheduling_basis: self.scheduling_basis.0.validate()?,
        })
        .map_err(|_| invalid())
    }
}

fn parse_time(value: &str) -> Result<HearingTime, ApiError> {
    let bytes = value.as_bytes();
    let shape = matches!(bytes.len(), 20 | 25)
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && [0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
            .into_iter()
            .all(|range| {
                bytes
                    .get(range)
                    .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
            })
        && ((bytes.len() == 20 && bytes[19] == b'Z')
            || (bytes.len() == 25
                && matches!(bytes[19], b'+' | b'-')
                && bytes[22] == b':'
                && bytes[20..22].iter().all(u8::is_ascii_digit)
                && bytes[23..25].iter().all(u8::is_ascii_digit)));
    if !shape || value.ends_with("-00:00") || &value[17..19] > "59" {
        return Err(invalid());
    }
    let at = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| invalid())?;
    HearingTime::new(at).map_err(|_| invalid())
}
