use application::judicial_calendars::*;
use serde_json::{json, Value};

pub(super) fn values(v: &JudicialCalendarValues) -> Value {
    let s = v.scope();
    let sources:Vec<_>=v.sources().iter().map(|s|json!({
        "id":s.id().to_string(),"title":s.title(),"issuer":s.issuer(),"official_url":s.official_url(),
        "published_on":s.published_on().map(|d|d.to_string()),"consulted_on":s.consulted_on().to_string(),"locator":s.locator()
    })).collect();
    let weekly: Vec<_> = v
        .weekly_pattern()
        .iter()
        .map(|w| {
            let mut result = rule(w.rule());
            result["weekday"] = json!(w.weekday());
            result
        })
        .collect();
    let exceptions: Vec<_> = v
        .exceptions()
        .iter()
        .map(|e| {
            let mut result = rule(e.rule());
            result["id"] = json!(e.id().to_string());
            result["from"] = json!(e.from().to_string());
            result["through"] = json!(e.through().to_string());
            result
        })
        .collect();
    json!({"scope":{"title":s.title(),"jurisdiction":s.jurisdiction().as_str(),"entity_codes":s.entity_codes(),
        "authority":s.authority(),"organ":s.organ(),"territory":s.territory(),"use_description":s.use_description()},
        "coverage":{"from":v.coverage().from().to_string(),"through":v.coverage().through().to_string()},
        "sources":sources,"weekly_pattern":weekly,"exceptions":exceptions})
}
fn rule(v: &JudicialCalendarRule) -> Value {
    json!({"classification":v.classification().as_str(),"source_ids":v.source_ids().iter().map(ToString::to_string).collect::<Vec<_>>(),"explanation":v.explanation()})
}
pub(super) fn receipt(v: &JudicialCalendarDetail) -> Value {
    json!({"actor_id":v.recorded_by.id.to_string(),"operation_id":v.receipt.operation_id.to_string(),
        "calendar_id":v.id.to_string(),"action":v.receipt.action.as_str(),"expected_revision":v.receipt.expected_revision,
        "values_digest":v.values_digest.to_hex(),"reason":v.reason.as_ref().map(JudicialCalendarReason::as_str)})
}
