use super::ID;
use serde_json::{json, Value};

pub fn definition_json(case: Option<&str>) -> Value {
    let scope = case.map(|id|json!({"kind":"case","case_id":id})).unwrap_or_else(||json!({"kind":"global","value":{
        "title":"Declared scope","jurisdiction":"federal","entity_codes":["09"],
        "authority":"Synthetic authority","organ":"Synthetic organ","territory":"Declared territory","use_description":"Test configuration"}}));
    json!({"title":"Synthetic rule","description":"Mathematical fixture without legal claims","scope":scope,
        "references":[{"id":ID,"title":"Synthetic reference","issuer":"Test fixture","official_url":"https://example.org/synthetic",
            "published_on":null,"consulted_on":"2026-01-06","locator":"Mathematical example"}],
        "trigger":{"kind":"source_field","field":"resolution_issued_at"},
        "template":{"kind":"fixed","rule":{"kind":"days","quantity":1,"inclusion":"on_anchor","basis":"natural","final_day":"preserve"}},
        "completion":{"kind":"civil_candidate_only"},
        "conditions":[{"id":"00000000-0000-0000-0000-000000000001","statement":"Operator must declare applicability","reference_ids":[ID]}],
        "examples":[{"id":"00000000-0000-0000-0000-000000000002","anchor":{"precision":"date","year":2026,"month":1,"day":6,"offset_seconds":null},
            "ordered_quantity":null,"calendar":null,"expected":{"kind":"arithmetic","outcome":{"kind":"civil_candidate","date":"2026-01-06"}},
            "reference_ids":[ID],"locator":"One included natural day"}]})
}
pub fn command_json(case: Option<&str>) -> Value {
    json!({"operation_id":ID,"profile_id":ID,"change":{"action":"publish","expected_revision":0,"definition":definition_json(case)}})
}
pub fn submission(action: &str, case: Option<&str>) -> Value {
    let mut command = command_json(case);
    if action != "publish" {
        command["change"]["action"] = json!(action);
        command["change"]["expected_revision"] = json!(1);
        command["change"]["reason"] = json!(" Explicit reason\r\nMore ");
        if action == "retire" {
            command["change"]
                .as_object_mut()
                .unwrap()
                .remove("definition");
        }
    }
    json!({"command":command,"expected_submission_digest":super::digest().to_hex()})
}
