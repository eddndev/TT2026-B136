use super::{definition::Definition, definition_projection};
use serde_json::{json, Value};

fn definition() -> Value {
    let id = "00000000-0000-0000-0000-000000000001";
    json!({
        "title":"Weekly reception", "description":"Declared ordinary reception channel",
        "scope":{"kind":"case","case_id":id},
        "references":[{"id":id,"title":"Reception reference","issuer":"Fixture",
            "official_url":"https://example.org/reception","published_on":null,
            "consulted_on":"2026-01-01","locator":"Declared weekly reception"}],
        "trigger":{"kind":"source_field","field":"notification_practiced_at"},
        "template":{"kind":"fixed","rule":{"kind":"days","quantity":1,
            "inclusion":"after_anchor","basis":"natural","final_day":"preserve"}},
        "completion":{"kind":"weekly_civil_cutoff",
            "times":["15:00:00","15:00:00","15:00:00","15:00:00","14:00:00",null,null],
            "offset_seconds":-21600,"from":"2026-01-01","through":"2026-12-31",
            "channel":"Ordinary reception","reference_id":id},
        "conditions":[{"id":id,"statement":"The declared channel applies","reference_ids":[id]}],
        "examples":[{"id":id,"anchor":{"precision":"date","year":2026,"month":10,"day":14,"offset_seconds":null},
            "ordered_quantity":null,"calendar":null,
            "expected":{"kind":"arithmetic","outcome":{"kind":"civil_candidate","date":"2026-10-15"}},
            "reference_ids":[id],"locator":"One natural day after the declared notification"}]
    })
}

fn assert_rejected(raw: Value) {
    if let Ok(parsed) = serde_json::from_value::<Definition>(raw) {
        assert!(parsed.validate().is_err());
    }
}

#[test]
fn weekly_cutoff_http_definition_roundtrip_preserves_monday_order_and_closed_days() {
    let raw = definition();
    let parsed: Definition = serde_json::from_value(raw.clone()).unwrap();
    let validated = parsed.validate().unwrap();
    let projected = definition_projection::definition(&validated).unwrap();
    assert_eq!(projected, raw);
    let reparsed: Definition = serde_json::from_value(projected).unwrap();
    assert_eq!(
        definition_projection::definition(&reparsed.validate().unwrap()).unwrap(),
        raw
    );
}

#[test]
fn weekly_cutoff_http_definition_rejects_invalid_week_shape_or_time() {
    let invalid_times = [
        json!(["15:00:00", "15:00:00", "15:00:00", "15:00:00", "14:00:00", null]),
        json!(["15:00:00", "15:00:00", "15:00:00", "15:00:00", "14:00:00", null, null, null]),
        json!([null, null, null, null, null, null, null]),
        json!(["24:00:00", null, null, null, null, null, null]),
        json!(["15:60:00", null, null, null, null, null, null]),
        json!(["15:00:60", null, null, null, null, null, null]),
        json!(["15:00", null, null, null, null, null, null]),
        json!(["15:00:00.1", null, null, null, null, null, null]),
        json!([150000, null, null, null, null, null, null]),
        json!(null),
    ];
    for times in invalid_times {
        let mut raw = definition();
        raw["completion"]["times"] = times;
        assert_rejected(raw);
    }
}

#[test]
fn weekly_cutoff_http_definition_rejects_unknown_fields_and_invalid_shared_metadata() {
    for (field, value) in [
        ("time", json!("15:00:00")),
        ("invented", json!(true)),
        ("offset_seconds", json!(-21601)),
        ("through", json!("2025-12-31")),
        ("channel", json!("")),
        (
            "reference_id",
            json!("00000000-0000-0000-0000-000000000099"),
        ),
    ] {
        let mut raw = definition();
        raw["completion"][field] = value;
        assert_rejected(raw);
    }
    let mut raw = definition();
    raw["template"]["rule"] = json!({"kind":"elapsed_hours","quantity":24});
    assert_rejected(raw);
}
