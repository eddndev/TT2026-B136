use super::super::{
    object::Object,
    request::{parse_uuid, revision},
};
use super::{
    catalog::*,
    checked, label_text,
    people::{Person, Representation},
    provenance::Provenance,
    text,
    time::DeclaredTime,
};
use crate::error::ApiError;
use application::procedural_facts::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Resolution {
    class: Object<Declaration<Object<Class>>>,
    subtype: Option<String>,
    issuer: Object<Declaration<String>>,
    issued_at: Object<DeclaredTime>,
    summary: String,
    provenance: Object<Provenance>,
}
impl Resolution {
    pub(in super::super) fn validate(self) -> Result<ResolutionValues, ApiError> {
        Ok(ResolutionValues::new(ResolutionValuesInput {
            class: self.class.0.validate(|v| v.0.validate())?,
            subtype: self.subtype.map(|v| label_text(&v)).transpose()?,
            issuer: self.issuer.0.validate(|v| label_text(&v))?,
            issued_at: self.issued_at.0.validate()?,
            summary: text(&self.summary)?,
            provenance: self.provenance.0.validate()?,
        }))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Notification {
    resolution: Object<ResolutionRef>,
    character: Object<Declaration<Object<Character>>>,
    medium: Object<Declaration<Object<Medium>>>,
    context: Object<Declaration<Object<Context>>>,
    outcome: Object<Declaration<Object<Outcome>>>,
    subtype: Option<String>,
    practiced_at: Object<DeclaredTime>,
    received_at: Option<Object<DeclaredTime>>,
    stated_effect: Option<Object<Effect>>,
    intended_recipient: Object<Declaration<Object<Person>>>,
    actual_receiver: Object<Declaration<Object<Person>>>,
    representation: Object<Representation>,
    summary: String,
    provenance: Object<Provenance>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolutionRef {
    id: String,
    revision: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Effect {
    at: Object<DeclaredTime>,
    statement: String,
    locator: String,
}
impl Effect {
    fn validate(self) -> Result<FactStatedEffect, ApiError> {
        Ok(FactStatedEffect {
            at: self.at.0.validate()?,
            statement: text(&self.statement)?,
            locator: label_text(&self.locator)?,
        })
    }
}
impl Notification {
    pub(in super::super) fn validate(self) -> Result<NotificationValues, ApiError> {
        checked(NotificationValues::new(NotificationValuesInput {
            resolution: FactResolutionRef {
                id: ResolutionId::from_uuid(parse_uuid(
                    &self.resolution.0.id,
                    "invalid_resolution_id",
                )?),
                revision: revision(self.resolution.0.revision)?,
            },
            character: self.character.0.validate(|v| v.0.validate())?,
            medium: self.medium.0.validate(|v| v.0.validate())?,
            context: self.context.0.validate(|v| v.0.validate())?,
            outcome: self.outcome.0.validate(|v| v.0.validate())?,
            subtype: self.subtype.map(|v| label_text(&v)).transpose()?,
            practiced_at: self.practiced_at.0.validate()?,
            received_at: self.received_at.map(|v| v.0.validate()).transpose()?,
            stated_effect: self.stated_effect.map(|v| v.0.validate()).transpose()?,
            intended_recipient: self.intended_recipient.0.validate(|v| v.0.validate())?,
            actual_receiver: self.actual_receiver.0.validate(|v| v.0.validate())?,
            representation: self.representation.0.validate()?,
            summary: text(&self.summary)?,
            provenance: self.provenance.0.validate()?,
        }))
    }
}
