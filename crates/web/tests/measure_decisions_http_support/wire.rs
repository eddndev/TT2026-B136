use super::*;
use domain::procedural_time::DeclaredProceduralPrecision as Precision;

pub fn command(value: &MeasureDecisionCommand) -> Value {
    let c = &value.context;
    let v = &value.values;
    let support = v.support();
    json!({"case_id":case_id().to_string(),"operation_id":value.operation_id.to_string(),
        "decision_id":value.decision_id.to_string(),"context":{
            "administration_revision":c.administration_revision.get(),"stage_revision":c.stage_revision.get(),"context_digest":c.context_digest.to_hex()},
        "values":{"authority":v.authority().as_str(),"declared_at":declared(v.declared_at()),
            "justification":v.justification().as_str(),"support":{"document_id":support.reference().id.to_string(),
                "version":support.reference().version.get(),"digest":support.digest().to_hex()},"locator":v.locator().as_str()},
        "anchor":value.anchor.as_ref().map(anchor),"outcome":outcome(&value.outcome)})
}
pub fn anchor(value: &MeasureDecisionAnchorRef) -> Value {
    match value {
        MeasureDecisionAnchorRef::Initial {
            hearing_id,
            revision,
            values_digest,
            submission_digest,
        } => {
            json!({"kind":"initial","hearing_id":hearing_id.to_string(),"revision":revision.get(),"values_digest":values_digest.to_hex(),"submission_digest":submission_digest.to_hex()})
        }
        MeasureDecisionAnchorRef::Precautionary {
            hearing_id,
            revision,
            capture_digest,
        } => {
            json!({"kind":"precautionary","hearing_id":hearing_id.to_string(),"revision":revision.get(),"capture_digest":capture_digest.to_hex()})
        }
    }
}
pub fn declared(value: &MeasureTime) -> Value {
    let d = value.declared();
    let precision = match d.precision() {
        Precision::Unknown => "unknown",
        Precision::Date => "date",
        Precision::Minute => "minute",
        Precision::Second => "second",
    };
    let mut result = json!({"precision":precision});
    if let Some(reason) = value.unknown_reason() {
        result["reason"] = json!(reason.as_str());
    }
    if let Some(date) = d.local_date() {
        result["year"] = json!(date.date().year());
        result["month"] = json!(date.date().month() as u8);
        result["day"] = json!(date.date().day());
        result["offset_seconds"] = json!(d.offset().map(|o| o.whole_seconds()));
    }
    if let Some(hour) = d.local_hour() {
        result["hour"] = json!(hour);
    }
    if let Some(minute) = d.local_minute() {
        result["minute"] = json!(minute);
    }
    if let Some(second) = d.local_second() {
        result["second"] = json!(second);
    }
    result
}
pub fn measure(value: &MeasureValues) -> Value {
    let s = value.subject();
    let supervision = match value.supervision() {
        MeasureSupervision::Known {
            participant,
            statement,
        } => {
            json!({"kind":"known","participant":{"participant_id":participant.id().to_string(),"revision":participant.revision().get()},"statement":statement.as_str()})
        }
        MeasureSupervision::Unknown { reason } => {
            json!({"kind":"unknown","reason":reason.as_str()})
        }
    };
    json!({"subject":{"id":s.id.to_string(),"revision":s.revision.get(),"values_digest":s.values_digest.to_hex()},
        "kind":value.kind().as_str(),"conditions":value.conditions().as_str(),"validity":{
            "start":declared(value.validity().start()),"statement":value.validity().statement().as_str(),"end":value.validity().end().map(declared)},"supervision":supervision})
}
pub fn reference(value: PrecautionaryMeasureRef) -> Value {
    json!({"id":value.id().to_string(),"revision":value.revision().get(),"capture_digest":value.digest().to_hex()})
}
pub fn proposal(value: &MeasureProposal) -> Value {
    json!({"id":value.id.to_string(),"values":measure(&value.values)})
}
pub fn outcome(value: &MeasureDecisionOutcome) -> Value {
    if let Some(statement) = value.no_measure_change() {
        return json!({"kind":"no_measure_change","statement":statement.as_str()});
    }
    json!({"kind":"changes","effects":value.changes().unwrap().iter().map(effect).collect::<Vec<_>>()})
}
pub fn effect(value: &MeasureEffect) -> Value {
    match value {
        MeasureEffect::Impose(v) => json!({"action":"impose","proposal":proposal(v)}),
        MeasureEffect::Confirm { previous } => {
            json!({"action":"confirm","previous":reference(*previous)})
        }
        MeasureEffect::Modify { previous, values } => {
            json!({"action":"modify","previous":reference(*previous),"values":measure(values)})
        }
        MeasureEffect::Revoke { previous } => {
            json!({"action":"revoke","previous":reference(*previous)})
        }
        MeasureEffect::Cease { previous } => {
            json!({"action":"cease","previous":reference(*previous)})
        }
        MeasureEffect::Substitute {
            predecessors,
            successors,
        } => {
            json!({"action":"substitute","predecessors":predecessors.iter().copied().map(reference).collect::<Vec<_>>(),"successors":successors.iter().map(proposal).collect::<Vec<_>>()})
        }
    }
}
