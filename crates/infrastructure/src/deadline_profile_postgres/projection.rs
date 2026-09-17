use super::inconsistent;
use application::{deadline_profiles::*, ApplicationError};
use domain::{
    cases::CaseId,
    judicial_calendars::{
        JudicialCalendarJurisdiction, JudicialCalendarScope, JudicialCalendarScopeInput,
    },
    procedural_facts::FactLabel,
};
use serde_json::{json, Value};

pub(super) fn scope(value: &DeadlineProfileScope) -> Value {
    match value {
        DeadlineProfileScope::Case(id) => json!({"kind":"case","case_id":id.to_string()}),
        DeadlineProfileScope::Global(s) => json!({"kind":"global","value":{
            "title":s.title(),"jurisdiction":s.jurisdiction().as_str(),"entity_codes":s.entity_codes(),
            "authority":s.authority(),"organ":s.organ(),"territory":s.territory(),"use_description":s.use_description()}}),
    }
}
pub(super) fn definition(v: &DeadlineProfileDefinition) -> Value {
    json!({"title":v.title().as_str(),"scope":scope(v.scope())})
}
pub(super) fn read(v: &Value) -> Result<(FactLabel, DeadlineProfileScope), ApplicationError> {
    let title = FactLabel::new(string(v, "title")?).map_err(inconsistent)?;
    let s = &v["scope"];
    let result = match string(s, "kind")? {
        "case" => DeadlineProfileScope::Case(CaseId::from_uuid(
            string(s, "case_id")?.parse().map_err(inconsistent)?,
        )),
        "global" => {
            let s = &s["value"];
            let codes = s["entity_codes"]
                .as_array()
                .ok_or_else(|| inconsistent("invalid profile entities"))?
                .iter()
                .map(|c| {
                    c.as_str()
                        .ok_or_else(|| inconsistent("invalid profile entity"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let jurisdiction = match string(s, "jurisdiction")? {
                "federal" => JudicialCalendarJurisdiction::Federal,
                "local" => JudicialCalendarJurisdiction::Local,
                _ => return Err(inconsistent("invalid profile jurisdiction")),
            };
            DeadlineProfileScope::Global(
                JudicialCalendarScope::new(JudicialCalendarScopeInput {
                    title: string(s, "title")?,
                    jurisdiction,
                    entity_codes: &codes,
                    authority: string(s, "authority")?,
                    organ: string(s, "organ")?,
                    territory: string(s, "territory")?,
                    use_description: string(s, "use_description")?,
                })
                .map_err(inconsistent)?,
            )
        }
        _ => return Err(inconsistent("invalid profile scope")),
    };
    if json!({"title":title.as_str(),"scope":scope(&result)}) != *v {
        return Err(inconsistent("noncanonical profile summary"));
    }
    Ok((title, result))
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, ApplicationError> {
    v[key]
        .as_str()
        .ok_or_else(|| inconsistent("invalid profile projection text"))
}
pub(super) fn receipt(v: &DeadlineProfileHistoryEntry) -> Value {
    json!({"actor_id":v.recorded_by.id.to_string(),"operation_id":v.receipt.operation_id.to_string(),
        "profile_id":v.id.to_string(),"action":v.receipt.action.as_str(),"expected_revision":v.receipt.expected_revision,
        "algorithm":1,"definition_digest":v.definition_digest.to_hex(),"reason":v.reason.as_ref().map(|r|r.as_str())})
}
