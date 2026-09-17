use super::super::{object::Object, request::parse_uuid};
use super::{
    checked, label_text,
    provenance::{self, Provenance},
    text,
};
use crate::error::ApiError;
use application::procedural_facts::*;
use domain::participants::{ParticipantId, ParticipantRevision};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Person {
    Participant { id: String, revision: u32 },
    Unlinked { label: String, description: String },
}
impl Person {
    pub(super) fn validate(self) -> Result<FactPerson, ApiError> {
        Ok(match self {
            Self::Participant { id, revision } => FactPerson::Participant(FactParticipantRef {
                id: ParticipantId::from_uuid(parse_uuid(&id, "invalid_participant_id")?),
                revision: checked(ParticipantRevision::new(revision))?,
            }),
            Self::Unlinked { label, description } => FactPerson::Unlinked {
                label: label_text(&label)?,
                description: text(&description)?,
            },
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Representation {
    NotRecorded {
        reason: String,
    },
    Declared {
        represented: Object<Person>,
        representative: Object<Person>,
        scope: String,
        provenance: Box<Object<Provenance>>,
    },
}
impl Representation {
    pub(super) fn validate(self) -> Result<FactRepresentation, ApiError> {
        Ok(match self {
            Self::NotRecorded { reason } => FactRepresentation::NotRecorded(text(&reason)?),
            Self::Declared {
                represented,
                representative,
                scope,
                provenance,
            } => FactRepresentation::Declared {
                represented: represented.0.validate()?,
                representative: representative.0.validate()?,
                scope: text(&scope)?,
                provenance: Box::new((*provenance).0.validate()?),
            },
        })
    }
}
pub(super) fn person(value: &FactPerson) -> Value {
    match value {
        FactPerson::Participant(v) => {
            json!({"kind":"participant","id":v.id.to_string(),"revision":v.revision.get()})
        }
        FactPerson::Unlinked { label, description } => {
            json!({"kind":"unlinked","label":label.as_str(),"description":description.as_str()})
        }
    }
}
pub(super) fn representation(value: &FactRepresentation) -> Value {
    match value {
        FactRepresentation::NotRecorded(reason) => {
            json!({"kind":"not_recorded","reason":reason.as_str()})
        }
        FactRepresentation::Declared {
            represented,
            representative,
            scope,
            provenance,
        } => json!({"kind":"declared",
            "represented":person(represented),"representative":person(representative),
            "scope":scope.as_str(),"provenance":provenance::project(provenance)}),
    }
}
