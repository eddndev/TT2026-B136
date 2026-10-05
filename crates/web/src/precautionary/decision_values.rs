use super::{
    measure_request::{invalid, Subject},
    measure_time::{DeclaredTime, Validity},
    primitives::{digest, uuid},
};
use crate::{error::ApiError, procedural_facts::object::Object};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearings::{HearingNote, HearingParticipantRef, HearingSupportRef},
    participants::{ParticipantId, ParticipantRevision},
    precautionary_hearings::MeasureId,
    precautionary_measures::*,
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
pub(super) struct Values {
    authority: String,
    declared_at: Object<DeclaredTime>,
    justification: String,
    support: Object<Support>,
    locator: String,
}
impl Values {
    pub(super) fn validate(self) -> Result<MeasureDecisionValues, ApiError> {
        Ok(MeasureDecisionValues::new(MeasureDecisionValuesInput {
            authority: note(&self.authority)?,
            declared_at: self.declared_at.0.validate()?,
            justification: note(&self.justification)?,
            support: self.support.0.validate()?,
            locator: note(&self.locator)?,
        }))
    }
}
fn note(value: &str) -> Result<HearingNote, ApiError> {
    HearingNote::new(value).map_err(|_| invalid())
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Supervision {
    Known {
        participant: Object<Participant>,
        statement: String,
    },
    Unknown {
        reason: String,
    },
}
impl Supervision {
    fn validate(self) -> Result<MeasureSupervision, ApiError> {
        Ok(match self {
            Self::Known {
                participant,
                statement,
            } => MeasureSupervision::Known {
                participant: participant.0.validate()?,
                statement: note(&statement)?,
            },
            Self::Unknown { reason } => MeasureSupervision::Unknown {
                reason: note(&reason)?,
            },
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Measure {
    subject: Object<Subject>,
    kind: String,
    conditions: String,
    validity: Object<Validity>,
    supervision: Object<Supervision>,
}
impl Measure {
    pub(super) fn validate(self) -> Result<MeasureValues, ApiError> {
        Ok(MeasureValues::new(MeasureValuesInput {
            subject: self.subject.0.validate()?,
            kind: self.kind.parse().map_err(|_| invalid())?,
            conditions: note(&self.conditions)?,
            validity: self.validity.0.validate()?,
            supervision: self.supervision.0.validate()?,
        }))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proposal {
    id: String,
    values: Object<Measure>,
}
impl Proposal {
    pub(super) fn validate(self) -> Result<MeasureProposal, ApiError> {
        Ok(MeasureProposal {
            id: MeasureId::from_uuid(uuid(&self.id)?),
            values: self.values.0.validate()?,
        })
    }
}
