use application::judicial_calendars::*;
use serde_json::{json, Value};
pub(crate) fn scope(v: &JudicialCalendarScope) -> Value {
    json!({"title":v.title(),"jurisdiction":v.jurisdiction().as_str(),"entity_codes":v.entity_codes(),"authority":v.authority(),"organ":v.organ(),"territory":v.territory(),"use_description":v.use_description()})
}
pub(super) fn coverage(v: JudicialCalendarCoverage) -> Value {
    json!({"from":v.from().to_string(),"through":v.through().to_string()})
}
pub(crate) fn values(v: &JudicialCalendarValues) -> Value {
    json!({"scope":scope(v.scope()),"coverage":coverage(v.coverage()),"sources":v.sources().iter().map(source).collect::<Vec<_>>(),"weekly_pattern":v.weekly_pattern().iter().map(|v|{let mut r=rule(v.rule());r["weekday"]=json!(v.weekday());r}).collect::<Vec<_>>(),"exceptions":v.exceptions().iter().map(|v|{let mut r=rule(v.rule());r["id"]=json!(v.id());r["from"]=json!(v.from().to_string());r["through"]=json!(v.through().to_string());r}).collect::<Vec<_>>()})
}
pub(crate) fn source(v: &JudicialCalendarSource) -> Value {
    json!({"id":v.id(),"title":v.title(),"issuer":v.issuer(),"official_url":v.official_url(),"published_on":v.published_on().map(|d|d.to_string()),"consulted_on":v.consulted_on().to_string(),"locator":v.locator()})
}
fn rule(v: &JudicialCalendarRule) -> Value {
    json!({"classification":v.classification().as_str(),"source_ids":v.source_ids(),"explanation":v.explanation()})
}
pub(super) fn command(v: &JudicialCalendarCommand) -> Value {
    let mut change =
        json!({"action":v.action().as_str(),"expected_revision":v.expected_revision()});
    if let Some(reason) = v.reason() {
        change["reason"] = json!(reason.as_str());
    }
    match &v.change {
        JudicialCalendarChange::Publish { values: v }
        | JudicialCalendarChange::Replace { values: v, .. } => change["values"] = values(v),
        _ => {}
    }
    json!({"operation_id":v.operation_id.to_string(),"calendar_id":v.calendar_id.to_string(),"change":change})
}
pub(super) fn receipt(v: &JudicialCalendarReceipt) -> Value {
    json!({"operation_id":v.operation_id.to_string(),"action":v.action.as_str(),"expected_revision":v.expected_revision,"submission_digest":v.submission_digest.to_hex()})
}
pub(super) fn day(v: &JudicialCalendarDay) -> Value {
    let mut result = json!({"date":v.date().to_string(),"state":v.classification().map(|c|c.as_str()).unwrap_or("outside_coverage"),"origin":null,"explanation":v.explanation(),"source_ids":v.source_ids()});
    match v.origin() {
        Some(JudicialCalendarDayOrigin::WeeklyPattern(day)) => {
            result["origin"] = json!("weekly_pattern");
            result["weekday"] = json!(day);
        }
        Some(JudicialCalendarDayOrigin::Exception(id)) => {
            result["origin"] = json!("exception");
            result["exception_id"] = json!(id);
        }
        None => {}
    }
    result
}
