#[path = "measure_correction_codec_support/strict.rs"]
mod strict;
#[path = "measure_correction_codec_support/mod.rs"]
mod support;
#[path = "measure_correction_codec_support/vectors.rs"]
mod vectors;

use domain::precautionary_measures::{MeasureCorrectionValues, MeasureValidity};
use domain::procedural_time::DeclaredProceduralTime as Declared;
use infrastructure::measure_correction_codec::{values, view};
use serde_json::json;
use support::*;
use time::UtcOffset;

#[test]
fn correction_values_round_trip_with_their_exact_mcval1_bytes() {
    let original = correction();
    let bytes = original.canonical_bytes();
    let decoded = values(&bytes, &view(&original)).unwrap();

    assert_eq!(decoded, original);
    assert_eq!(decoded.canonical_bytes(), bytes);
}

#[test]
fn projection_contains_only_the_three_correction_values_and_declared_validity() {
    assert_eq!(
        view(&correction()),
        json!({
            "conditions":"C",
            "validity":{
                "start":{"precision":"unknown","reason":"U"},
                "statement":"V","end":null
            },
            "supervision_text":"S"
        }),
    );
}

#[test]
fn known_precisions_preserve_absent_and_explicit_offsets_and_end_components() {
    for offset in [
        None,
        Some(UtcOffset::UTC),
        Some(UtcOffset::from_hms(-14, 0, 0).unwrap()),
        Some(UtcOffset::from_hms(5, 45, 0).unwrap()),
        Some(UtcOffset::from_hms(14, 0, 0).unwrap()),
    ] {
        let day = "2024-02-29".parse().unwrap();
        for declared in [
            Declared::date(day, offset).unwrap(),
            Declared::minute(day, 23, 59, offset).unwrap(),
            Declared::second(day, 23, 59, 59, offset).unwrap(),
        ] {
            let value = with_times(known(declared), Some(known(declared)));
            let projection = view(&value);
            assert_eq!(
                projection["validity"]["start"]["offset_seconds"],
                json!(offset.map(UtcOffset::whole_seconds)),
            );
            assert_eq!(
                values(&value.canonical_bytes(), &projection).unwrap(),
                value
            );
        }
    }
}

#[test]
fn absent_end_and_explicit_unknown_end_remain_different_values() {
    let absent = correction();
    let explicit = with_times(unknown("U"), Some(unknown("E")));
    assert_ne!(absent.canonical_bytes(), explicit.canonical_bytes());
    assert!(view(&absent)["validity"]["end"].is_null());
    assert_eq!(
        view(&explicit)["validity"]["end"],
        json!({"precision":"unknown","reason":"E"}),
    );
    assert_eq!(
        values(&explicit.canonical_bytes(), &view(&explicit)).unwrap(),
        explicit,
    );
}

#[test]
fn boundary_years_and_incomparable_precisions_round_trip_without_inferred_instants() {
    for date in ["0001-01-01", "9999-12-31"] {
        for offset in [None, Some(UtcOffset::UTC)] {
            let value = with_times(
                known(Declared::date(date.parse().unwrap(), offset).unwrap()),
                None,
            );
            assert_eq!(
                values(&value.canonical_bytes(), &view(&value)).unwrap(),
                value
            );
        }
    }
    let value = with_times(
        known(Declared::second("2026-10-04".parse().unwrap(), 1, 2, 3, None).unwrap()),
        Some(known(
            Declared::date("2025-01-01".parse().unwrap(), None).unwrap(),
        )),
    );
    assert_eq!(
        values(&value.canonical_bytes(), &view(&value)).unwrap(),
        value
    );
}

#[test]
fn canonical_multiline_and_unicode_text_survive_json_transport_exactly() {
    for text in ["A\n\\\"".repeat(250), "A\u{e9}\u{1f642}\nB\\\"".into()] {
        let value = MeasureCorrectionValues::new(
            note(&text),
            MeasureValidity::new(unknown(&text), note(&text), Some(unknown(&text))).unwrap(),
            note(&text),
        );
        let encoded = serde_json::to_vec(&view(&value)).unwrap();
        assert!(encoded.len() <= 32_768);
        let transported = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            values(&value.canonical_bytes(), &transported).unwrap(),
            value
        );
        assert_eq!(value.conditions().as_str(), text);
    }
}
