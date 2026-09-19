use super::{
    object::{nullable, Object},
    request::{checked, uuid},
};
use crate::{
    error::ApiError,
    procedural_facts::values::{
        catalog::{declaration, Declaration},
        provenance::{evidence, Evidence},
        time::{project, DeclaredTime},
    },
};
use application::procedural_resources::*;
use domain::{
    participants::{ParticipantId, ParticipantRevision},
    procedural_facts::*,
};
use serde::Deserialize;
use serde_json::{json, Value};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Values {
    kind: String,
    mode: Object<Declaration<String>>,
    title: String,
    resolution: Object<Resolution>,
    resolution_evidence: Object<Evidence>,
    resolution_reference: Object<Declaration<String>>,
    issuing_authority: Object<Declaration<String>>,
    #[serde(deserialize_with = "nullable")]
    receiving_authority: Option<Object<Declaration<String>>>,
    resolution_at: Object<DeclaredTime>,
    #[serde(deserialize_with = "nullable")]
    notification_at: Option<Object<DeclaredTime>>,
    challenged_part: String,
    grounds: String,
    appellants: Vec<Object<Appellant>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolution {
    id: String,
    revision: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Participant {
    id: String,
    revision: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Appellant {
    name: String,
    role: Object<Declaration<String>>,
    #[serde(deserialize_with = "nullable")]
    participant: Option<Object<Participant>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ActValues {
    kind: String,
    mode: Object<Declaration<String>>,
    occurred_at: Object<DeclaredTime>,
    authority: Object<Declaration<String>>,
    statement: String,
    evidence: Vec<Object<Evidence>>,
}
fn label(v: String) -> Result<FactLabel, ApiError> {
    checked(FactLabel::new(&v))
}
pub(super) fn text(v: String) -> Result<FactText, ApiError> {
    checked(FactText::new(&v))
}
impl Values {
    pub(super) fn validate(self) -> Result<ResourceValues, ApiError> {
        checked(ResourceValues::new(ResourceValuesInput {
            kind: checked(self.kind.parse())?,
            mode: self.mode.0.validate(|v| checked(v.parse()))?,
            title: label(self.title)?,
            resolution: FactResolutionRef {
                id: ResolutionId::from_uuid(uuid(&self.resolution.0.id)?),
                revision: checked(FactRevision::new(self.resolution.0.revision))?,
            },
            resolution_evidence: self.resolution_evidence.0.validate()?,
            resolution_reference: self.resolution_reference.0.validate(label)?,
            issuing_authority: self.issuing_authority.0.validate(label)?,
            receiving_authority: self
                .receiving_authority
                .map(|v| v.0.validate(label))
                .transpose()?,
            resolution_at: self.resolution_at.0.validate()?,
            notification_at: self.notification_at.map(|v| v.0.validate()).transpose()?,
            challenged_part: text(self.challenged_part)?,
            grounds: text(self.grounds)?,
            appellants: self
                .appellants
                .into_iter()
                .map(|v| {
                    let v = v.0;
                    Ok(ResourceAppellant::new(
                        label(v.name)?,
                        v.role.0.validate(label)?,
                        v.participant
                            .map(|p| {
                                Ok::<_, ApiError>(FactParticipantRef {
                                    id: ParticipantId::from_uuid(uuid(&p.0.id)?),
                                    revision: checked(ParticipantRevision::new(p.0.revision))?,
                                })
                            })
                            .transpose()?,
                    ))
                })
                .collect::<Result<_, ApiError>>()?,
        }))
    }
}
impl ActValues {
    pub(super) fn validate(self) -> Result<ResourceActValues, ApiError> {
        checked(ResourceActValues::new(ResourceActValuesInput {
            kind: checked(self.kind.parse())?,
            mode: self.mode.0.validate(|v| checked(v.parse()))?,
            occurred_at: self.occurred_at.0.validate()?,
            authority: self.authority.0.validate(label)?,
            statement: text(self.statement)?,
            evidence: self
                .evidence
                .into_iter()
                .map(|v| v.0.validate())
                .collect::<Result<_, _>>()?,
        }))
    }
}
pub(super) fn values(v: &ResourceValues) -> Result<Value, ApiError> {
    Ok(
        json!({"kind":v.kind().as_str(),"mode":declaration(v.mode(),|v|json!(v.as_str())),
        "title":v.title().as_str(),"resolution":{"id":v.resolution().id.to_string(),"revision":v.resolution().revision.get()},
        "resolution_evidence":evidence(v.resolution_evidence()),
        "resolution_reference":declaration(v.resolution_reference(),|v|json!(v.as_str())),
        "issuing_authority":declaration(v.issuing_authority(),|v|json!(v.as_str())),
        "receiving_authority":v.receiving_authority().map(|v|declaration(v,|v|json!(v.as_str()))),
        "resolution_at":project(v.resolution_at())?,"notification_at":v.notification_at().map(project).transpose()?,
        "challenged_part":v.challenged_part().as_str(),"grounds":v.grounds().as_str(),
        "appellants":v.appellants().iter().map(|p|json!({"name":p.name().as_str(),
            "role":declaration(p.role(),|v|json!(v.as_str())),
            "participant":p.participant().map(|v|json!({"id":v.id.to_string(),"revision":v.revision.get()}))})).collect::<Vec<_>>() }),
    )
}
pub(super) fn act(v: &ResourceActValues) -> Result<Value, ApiError> {
    Ok(
        json!({"kind":v.kind().as_str(),"mode":declaration(v.mode(),|v|json!(v.as_str())),
        "occurred_at":project(v.occurred_at())?,"authority":declaration(v.authority(),|v|json!(v.as_str())),
        "statement":v.statement().as_str(),"evidence":v.evidence().iter().map(evidence).collect::<Vec<_>>() }),
    )
}
