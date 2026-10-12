use super::{definition::Definition, definition_projection};
use serde_json::{json, Value};

fn definition() -> Value {
    let id = "00000000-0000-0000-0000-000000000001";
    json!({
        "title":"Declared hearing end", "description":"Exact declared end without a duration",
        "scope":{"kind":"case","case_id":id},
        "references":[{"id":id,"title":"Declared reference","issuer":"Fixture",
            "official_url":"https://example.org/fixture","published_on":null,
            "consulted_on":"2026-01-01","locator":"Declared hearing end"}],
        "trigger":{"kind":"qualified","purpose":"hearing_end","family":"hearing_result"},
        "template":{"kind":"fixed","rule":{"kind":"at_anchor"}},
        "completion":{"kind":"arithmetic_instant"},
        "conditions":[{"id":id,"statement":"The selected hearing end applies","reference_ids":[id]}],
        "examples":[{"id":id,"anchor":{"precision":"second","year":2026,"month":1,"day":1,
            "hour":0,"minute":0,"second":7,"offset_seconds":-21600},
            "ordered_quantity":null,"calendar":null,
            "expected":{"kind":"arithmetic","outcome":{"kind":"instant_candidate",
                "instant":{"unix_seconds":1767247207_i64,"nanosecond":0,"offset_seconds":-21600}}},
            "reference_ids":[id],"locator":"Exact end with original offset"}]
    })
}

#[test]
fn at_anchor_http_definition_preserves_exact_hearing_end_without_quantity() {
    let raw = definition();
    let parsed: Definition = serde_json::from_value(raw.clone()).unwrap();
    let validated = parsed.validate().unwrap();
    assert_eq!(definition_projection::definition(&validated).unwrap(), raw);
}

#[test]
fn at_anchor_http_definition_rejects_duration_and_incompatible_trigger_or_completion() {
    let cases = [
        ("/template/rule", json!({"kind":"at_anchor","quantity":0})),
        ("/template/rule", json!({"kind":"at_anchor","quantity":1})),
        (
            "/template/rule",
            json!({"kind":"at_anchor","final_day":"preserve"}),
        ),
        (
            "/template",
            json!({"kind":"ordered","unit":{"kind":"at_anchor"},"maximum":null}),
        ),
        (
            "/trigger",
            json!({"kind":"source_field","field":"hearing_session_event_time"}),
        ),
        ("/trigger/purpose", json!("ordered_period_start")),
        ("/trigger/family", json!("resolution")),
        ("/trigger/family", json!("notification")),
        ("/completion", json!({"kind":"civil_candidate_only"})),
    ];
    for (pointer, value) in cases {
        let mut raw = definition();
        *raw.pointer_mut(pointer).unwrap() = value;
        if let Ok(parsed) = serde_json::from_value::<Definition>(raw) {
            assert!(
                parsed.validate().is_err(),
                "accepted invalid field {pointer}"
            );
        }
    }
}
