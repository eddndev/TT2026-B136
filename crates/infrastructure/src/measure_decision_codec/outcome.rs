use super::{
    primitives::*,
    values::{decode_measure, measure_view},
    Result,
};
use domain::{
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
    precautionary_measures::*,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use uuid::Uuid;

pub fn outcome(bytes: &[u8], projection: &Value) -> Result<MeasureDecisionOutcome> {
    frame(bytes, b"MEFX1", 11, 645450)?;
    let input = match string(&projection["kind"])? {
        "no_measure_change" => {
            fields(projection, &["kind", "statement"])?;
            MeasureDecisionOutcomeInput::NoMeasureChange(note(&projection["statement"])?)
        }
        "changes" => {
            fields(projection, &["kind", "effects"])?;
            let effects = array(&projection["effects"])?;
            preflight(effects)?;
            MeasureDecisionOutcomeInput::Changes(effects.iter().map(effect).collect::<Result<_>>()?)
        }
        _ => return Err(inconsistent()),
    };
    let value = MeasureDecisionOutcome::new(input).map_err(|_| inconsistent())?;
    if value.canonical_bytes() != bytes {
        return Err(inconsistent());
    }
    Ok(value)
}

fn array(value: &Value) -> Result<&[Value]> {
    let values = value.as_array().ok_or_else(inconsistent)?;
    if values.is_empty() || values.len() > 32 {
        return Err(inconsistent());
    }
    Ok(values)
}

// Count the complete affected set before allocating or decoding any nested values.
fn preflight(effects: &[Value]) -> Result<()> {
    let mut count = 0_usize;
    for item in effects {
        let size = match string(&item["action"])? {
            "impose" => {
                fields(item, &["action", "proposal"])?;
                1
            }
            "confirm" | "revoke" | "cease" => {
                fields(item, &["action", "previous"])?;
                1
            }
            "modify" => {
                fields(item, &["action", "previous", "values"])?;
                1
            }
            "substitute" => {
                fields(item, &["action", "predecessors", "successors"])?;
                array(&item["predecessors"])?.len() + array(&item["successors"])?.len()
            }
            _ => return Err(inconsistent()),
        };
        count = count.checked_add(size).ok_or_else(inconsistent)?;
        if count > 32 {
            return Err(inconsistent());
        }
    }
    let mut identities = BTreeSet::new();
    let mut previous_key = None;
    for item in effects {
        let mut ids = Vec::new();
        match string(&item["action"])? {
            "impose" => ids.push(proposal_id(&item["proposal"])?),
            "substitute" => {
                let before = array(&item["predecessors"])?
                    .iter()
                    .map(|v| reference(v).map(|r| r.id().as_uuid()))
                    .collect::<Result<Vec<_>>>()?;
                let after = array(&item["successors"])?
                    .iter()
                    .map(proposal_id)
                    .collect::<Result<Vec<_>>>()?;
                for side in [&before, &after] {
                    if side.windows(2).any(|pair| pair[0] >= pair[1]) {
                        return Err(inconsistent());
                    }
                }
                ids.extend(before);
                ids.extend(after);
            }
            _ => ids.push(reference(&item["previous"])?.id().as_uuid()),
        }
        let key = *ids.iter().min().ok_or_else(inconsistent)?;
        if previous_key.is_some_and(|previous| previous >= key) {
            return Err(inconsistent());
        }
        previous_key = Some(key);
        for id in ids {
            if !identities.insert(id) {
                return Err(inconsistent());
            }
        }
    }
    Ok(())
}

fn reference(value: &Value) -> Result<PrecautionaryMeasureRef> {
    fields(value, &["id", "revision", "digest"])?;
    Ok(PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(uuid(&value["id"])?),
        MeasureRevision::new(integer(&value["revision"])?).map_err(|_| inconsistent())?,
        digest(&value["digest"])?,
    ))
}
fn proposal_id(value: &Value) -> Result<Uuid> {
    fields(value, &["id", "values"])?;
    uuid(&value["id"])
}
fn proposal(value: &Value) -> Result<MeasureProposal> {
    Ok(MeasureProposal {
        id: MeasureId::from_uuid(proposal_id(value)?),
        values: decode_measure(&value["values"])?,
    })
}
fn effect(value: &Value) -> Result<MeasureEffect> {
    Ok(match string(&value["action"])? {
        "impose" => MeasureEffect::Impose(proposal(&value["proposal"])?),
        "confirm" => MeasureEffect::Confirm {
            previous: reference(&value["previous"])?,
        },
        "modify" => MeasureEffect::Modify {
            previous: reference(&value["previous"])?,
            values: decode_measure(&value["values"])?,
        },
        "revoke" => MeasureEffect::Revoke {
            previous: reference(&value["previous"])?,
        },
        "cease" => MeasureEffect::Cease {
            previous: reference(&value["previous"])?,
        },
        "substitute" => MeasureEffect::Substitute {
            predecessors: array(&value["predecessors"])?
                .iter()
                .map(reference)
                .collect::<Result<_>>()?,
            successors: array(&value["successors"])?
                .iter()
                .map(proposal)
                .collect::<Result<_>>()?,
        },
        _ => return Err(inconsistent()),
    })
}

fn reference_view(value: &PrecautionaryMeasureRef) -> Value {
    json!({"id":value.id().to_string(),"revision":value.revision().get(),"digest":value.digest().to_hex()})
}
fn proposal_view(value: &MeasureProposal) -> Value {
    json!({"id":value.id.to_string(),"values":measure_view(&value.values)})
}
fn effect_view(value: &MeasureEffect) -> Value {
    match value {
        MeasureEffect::Impose(value) => json!({"action":"impose","proposal":proposal_view(value)}),
        MeasureEffect::Confirm { previous } => {
            json!({"action":"confirm","previous":reference_view(previous)})
        }
        MeasureEffect::Modify { previous, values } => {
            json!({"action":"modify","previous":reference_view(previous),"values":measure_view(values)})
        }
        MeasureEffect::Revoke { previous } => {
            json!({"action":"revoke","previous":reference_view(previous)})
        }
        MeasureEffect::Cease { previous } => {
            json!({"action":"cease","previous":reference_view(previous)})
        }
        MeasureEffect::Substitute {
            predecessors,
            successors,
        } => json!({"action":"substitute",
            "predecessors":predecessors.iter().map(reference_view).collect::<Vec<_>>(),
            "successors":successors.iter().map(proposal_view).collect::<Vec<_>>()}),
    }
}
pub fn outcome_view(value: &MeasureDecisionOutcome) -> Value {
    match value.changes() {
        Some(effects) => {
            json!({"kind":"changes","effects":effects.iter().map(effect_view).collect::<Vec<_>>()})
        }
        None => {
            json!({"kind":"no_measure_change","statement":value.no_measure_change().expect("checked outcome has one variant").as_str()})
        }
    }
}
