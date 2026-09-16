use super::{
    request::{parse_digest, parse_uuid},
    time::DeclaredTime,
};
use crate::error::ApiError;
use application::{hearing_results::*, ApplicationError};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    participants::{ParticipantId, ParticipantRevision},
};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Values {
    occurrence: String,
    extent: String,
    #[serde(deserialize_with = "super::object::deserialize")]
    event_time: DeclaredTime,
    summary: String,
    #[serde(deserialize_with = "super::object::array")]
    attendees: Vec<Attendee>,
    #[serde(deserialize_with = "super::object::array")]
    agreements: Vec<Agreement>,
    #[serde(deserialize_with = "super::object::deserialize")]
    provenance: Provenance,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Attendee {
    participant_id: String,
    revision: u32,
    capacity: String,
    observation: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Agreement {
    id: String,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    kind: String,
    reference: Option<String>,
    #[serde(default, deserialize_with = "super::object::optional")]
    support: Option<Support>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Support {
    document_id: String,
    version: u32,
    digest: String,
}
impl Values {
    pub fn validate(self) -> Result<HearingResultValues, ApiError> {
        HearingResultValues::new(HearingResultValuesInput {
            occurrence: self.occurrence.parse().map_err(ApplicationError::from)?,
            extent: self.extent.parse().map_err(ApplicationError::from)?,
            event_time: self.event_time.validate().map_err(ApplicationError::from)?,
            summary: HearingResultText::new(&self.summary).map_err(ApplicationError::from)?,
            attendees: self
                .attendees
                .into_iter()
                .map(Attendee::validate)
                .collect::<Result<_, _>>()?,
            agreements: self
                .agreements
                .into_iter()
                .map(Agreement::validate)
                .collect::<Result<_, _>>()?,
            provenance: self.provenance.validate()?,
        })
        .map_err(|e| ApplicationError::from(e).into())
    }
}
impl Attendee {
    fn validate(self) -> Result<HearingResultAttendee, ApiError> {
        Ok(HearingResultAttendee::new(
            ParticipantId::from_uuid(parse_uuid(&self.participant_id, "invalid_participant_id")?),
            ParticipantRevision::new(self.revision).map_err(ApplicationError::from)?,
            HearingResultCapacity::new(&self.capacity).map_err(ApplicationError::from)?,
            HearingResultObservation::optional(self.observation.as_deref())
                .map_err(ApplicationError::from)?,
        ))
    }
}
impl Agreement {
    fn validate(self) -> Result<HearingResultAgreement, ApiError> {
        Ok(HearingResultAgreement::new(
            HearingResultAgreementId::from_uuid(parse_uuid(
                &self.id,
                "invalid_hearing_result_agreement_id",
            )?),
            HearingResultText::new(&self.text).map_err(ApplicationError::from)?,
        ))
    }
}
impl Provenance {
    fn validate(self) -> Result<HearingResultProvenance, ApiError> {
        HearingResultProvenance::new(
            self.kind.parse().map_err(ApplicationError::from)?,
            HearingResultReference::optional(self.reference.as_deref())
                .map_err(ApplicationError::from)?,
            self.support.map(Support::validate).transpose()?,
        )
        .map_err(|e| ApplicationError::from(e).into())
    }
}
impl Support {
    fn validate(self) -> Result<HearingResultSupportRef, ApiError> {
        Ok(HearingResultSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(parse_uuid(&self.document_id, "invalid_document_id")?),
                version: DocumentVersion::new(self.version)
                    .map_err(|_| ApiError::invalid_document_version())?,
            },
            parse_digest(&self.digest)?,
        ))
    }
}
