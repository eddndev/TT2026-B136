use super::{invalid, primitives::*, Result};
use application::{procedural_facts::*, procedural_resources::*};
use domain::participants::{ParticipantId, ParticipantRevision};
use serde_json::{json, Value};

pub(crate) fn encode_values(v: &ResourceValues) -> Value {
    json!({"kind":v.kind().as_str(),"mode":declaration(v.mode(),|v|json!(v.as_str())),
        "title":v.title().as_str(),"resolution":{"id":v.resolution().id.as_uuid().to_string(),"revision":v.resolution().revision.get()},
        "resolution_evidence":evidence(v.resolution_evidence()),
        "resolution_reference":declaration(v.resolution_reference(),|v|json!(v.as_str())),
        "issuing_authority":declaration(v.issuing_authority(),|v|json!(v.as_str())),
        "receiving_authority":v.receiving_authority().map(|v|declaration(v,|v|json!(v.as_str()))),
        "resolution_at":time(v.resolution_at()),"notification_at":v.notification_at().map(time),
        "challenged_part":v.challenged_part().as_str(),"grounds":v.grounds().as_str(),
        "appellants":v.appellants().iter().map(|v|json!({"name":v.name().as_str(),"role":declaration(v.role(),|v|json!(v.as_str())),
            "participant":v.participant().map(|r|json!({"id":r.id.as_uuid().to_string(),"revision":r.revision.get()}))})).collect::<Vec<_>>()})
}
pub(crate) fn decode_values(v: &Value) -> Result<ResourceValues> {
    let values = ResourceValues::new(ResourceValuesInput {
        kind: string(&v["kind"])?.parse().map_err(|_| invalid())?,
        mode: declared(&v["mode"], |v| string(v)?.parse().map_err(|_| invalid()))?,
        title: label(&v["title"])?,
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(uuid(&v["resolution"]["id"])?),
            revision: FactRevision::new(number(&v["resolution"]["revision"])?)
                .map_err(|_| invalid())?,
        },
        resolution_evidence: decode_evidence(&v["resolution_evidence"])?,
        resolution_reference: declared(&v["resolution_reference"], label)?,
        issuing_authority: declared(&v["issuing_authority"], label)?,
        receiving_authority: optional(&v["receiving_authority"], |v| declared(v, label))?,
        resolution_at: crate::procedural_fact_codec::declared_time(&v["resolution_at"])?,
        notification_at: optional(
            &v["notification_at"],
            crate::procedural_fact_codec::declared_time,
        )?,
        challenged_part: text(&v["challenged_part"])?,
        grounds: text(&v["grounds"])?,
        appellants: array(&v["appellants"], 32)?
            .iter()
            .map(|v| {
                Ok(ResourceAppellant::new(
                    label(&v["name"])?,
                    declared(&v["role"], label)?,
                    optional(&v["participant"], |v| {
                        Ok(FactParticipantRef {
                            id: ParticipantId::from_uuid(uuid(&v["id"])?),
                            revision: ParticipantRevision::new(number(&v["revision"])?)
                                .map_err(|_| invalid())?,
                        })
                    })?,
                ))
            })
            .collect::<Result<Vec<_>>>()?,
    })
    .map_err(|_| invalid())?;
    if encode_values(&values) != *v {
        return Err(invalid());
    }
    Ok(values)
}
pub(crate) fn encode_act(v: &ResourceActValues) -> Value {
    json!({"kind":v.kind().as_str(),"mode":declaration(v.mode(),|v|json!(v.as_str())),"occurred_at":time(v.occurred_at()),
        "authority":declaration(v.authority(),|v|json!(v.as_str())),"statement":v.statement().as_str(),"evidence":v.evidence().iter().map(evidence).collect::<Vec<_>>()})
}
pub(crate) fn decode_act(v: &Value) -> Result<ResourceActValues> {
    let values = ResourceActValues::new(ResourceActValuesInput {
        kind: string(&v["kind"])?.parse().map_err(|_| invalid())?,
        mode: declared(&v["mode"], |v| string(v)?.parse().map_err(|_| invalid()))?,
        occurred_at: crate::procedural_fact_codec::declared_time(&v["occurred_at"])?,
        authority: declared(&v["authority"], label)?,
        statement: text(&v["statement"])?,
        evidence: array(&v["evidence"], 2)?
            .iter()
            .map(decode_evidence)
            .collect::<Result<Vec<_>>>()?,
    })
    .map_err(|_| invalid())?;
    if encode_act(&values) != *v {
        return Err(invalid());
    }
    Ok(values)
}
