use crate::*;
use serde_json::Value;

fn rejects(value: &MeasureCorrectionValues, change: impl FnOnce(&mut Value)) {
    let mut projection = view(value);
    change(&mut projection);
    assert!(values(&value.canonical_bytes(), &projection).is_err());
}

#[test]
fn frame_bounds_tags_lengths_truncation_and_trailing_bytes_are_strict() {
    let original = correction();
    let bytes = original.canonical_bytes();
    let projection = view(&original);
    for length in 0..bytes.len() {
        assert!(values(&bytes[..length], &projection).is_err());
    }
    for (offset, replacement) in [
        (0, b'X'),
        (5, b'2'),
        (15, b'X'),
        (19, b'2'),
        (6, 255),
        (14, 17),
        (14, 19),
        (20, 4),
        (21, 0),
        (21, 2),
        (32, 2),
    ] {
        let mut malformed = bytes.clone();
        malformed[offset] = replacement;
        assert!(values(&malformed, &projection).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(values(&trailing, &projection).is_err());
    let mut oversized = b"MCVAL1".to_vec();
    oversized.resize(20_041, 0);
    assert!(values(&oversized, &Value::Null).is_err());
}

#[test]
fn every_projected_object_has_exact_keys_and_scalar_shapes() {
    let value = with_times(unknown("U"), Some(unknown("E")));
    for path in ["", "/validity", "/validity/start", "/validity/end"] {
        let original = view(&value);
        let keys: Vec<_> = original
            .pointer(path)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for key in keys {
            rejects(&value, |v| {
                v.pointer_mut(path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(&key);
            });
        }
        rejects(&value, |v| {
            v.pointer_mut(path).unwrap()["unexpected"] = json!(true);
        });
        for wrong in [Value::Null, json!([]), json!("object"), json!(1)] {
            rejects(&value, |v| *v.pointer_mut(path).unwrap() = wrong);
        }
    }
}

#[test]
fn normalization_and_text_limit_violations_are_rejected_in_every_note() {
    let value = with_times(unknown("U"), Some(unknown("E")));
    for path in [
        "/conditions",
        "/supervision_text",
        "/validity/statement",
        "/validity/start/reason",
        "/validity/end/reason",
    ] {
        let original = view(&value)
            .pointer(path)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        for changed in [format!(" {original}"), format!("{original} ")] {
            rejects(&value, |v| *v.pointer_mut(path).unwrap() = json!(changed));
        }
        for invalid in [
            "".into(),
            " ".into(),
            " leading".into(),
            "trailing ".into(),
            "A\r\nB".into(),
            "A\tB".into(),
            "A\0B".into(),
            "A".repeat(1001),
            "\u{1f642}".repeat(1001),
        ] {
            rejects(&value, |v| *v.pointer_mut(path).unwrap() = json!(invalid));
        }
        for wrong in [Value::Null, json!(true), json!(4), json!([])] {
            rejects(&value, |v| *v.pointer_mut(path).unwrap() = wrong);
        }
    }
    let multiline = MeasureCorrectionValues::new(
        note("A\nB"),
        MeasureValidity::new(unknown("A\nB"), note("A\nB"), Some(unknown("A\nB"))).unwrap(),
        note("A\nB"),
    );
    for path in [
        "/conditions",
        "/supervision_text",
        "/validity/statement",
        "/validity/start/reason",
        "/validity/end/reason",
    ] {
        rejects(&multiline, |v| {
            *v.pointer_mut(path).unwrap() = json!("A\r\nB")
        });
    }
}

#[test]
fn precision_variants_cannot_gain_or_lose_components_or_normalize_names() {
    let unknown = correction();
    rejects(&unknown, |v| {
        v["validity"]["start"]["offset_seconds"] = Value::Null
    });
    rejects(&unknown, |v| v["validity"]["start"]["year"] = json!(2026));
    for precision in ["Unknown", " unknown", "instant", "", "DATE"] {
        rejects(&unknown, |v| {
            v["validity"]["start"]["precision"] = json!(precision)
        });
    }
    let day = "2026-10-04".parse().unwrap();
    for declared in [
        Declared::date(day, None).unwrap(),
        Declared::minute(day, 1, 2, None).unwrap(),
        Declared::second(day, 1, 2, 3, Some(UtcOffset::UTC)).unwrap(),
    ] {
        let value = with_times(known(declared), None);
        rejects(&value, |v| {
            v["validity"]["start"]["reason"] = json!("Invented")
        });
        rejects(&value, |v| v["validity"]["start"]["nanoseconds"] = json!(0));
        let keys: Vec<_> = view(&value)["validity"]["start"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for key in keys {
            rejects(&value, |v| {
                v["validity"]["start"].as_object_mut().unwrap().remove(&key);
            });
        }
    }
}

#[test]
fn invalid_calendar_components_offsets_and_numeric_coercions_are_rejected() {
    let value = with_times(
        known(Declared::second("2026-10-04".parse().unwrap(), 1, 2, 3, None).unwrap()),
        None,
    );
    for (key, invalid) in [
        ("year", 0),
        ("year", 10000),
        ("month", 0),
        ("month", 13),
        ("day", 0),
        ("day", 32),
        ("hour", 24),
        ("minute", 60),
        ("second", 60),
    ] {
        rejects(&value, |v| v["validity"]["start"][key] = json!(invalid));
    }
    rejects(&value, |v| {
        v["validity"]["start"]["month"] = json!(2);
        v["validity"]["start"]["day"] = json!(29);
    });
    for invalid in [
        json!(1),
        json!(-50401),
        json!(50401),
        json!(i64::MAX),
        json!(0.0),
        json!("0"),
    ] {
        rejects(&value, |v| {
            v["validity"]["start"]["offset_seconds"] = invalid
        });
    }
    for key in ["year", "month", "day", "hour", "minute", "second"] {
        for wrong in [json!(-1), json!(1.0), json!("1"), Value::Null] {
            rejects(&value, |v| v["validity"]["start"][key] = wrong);
        }
    }
}

#[test]
fn valid_but_different_projected_values_cannot_replace_the_committed_bytes() {
    let value = correction();
    rejects(&value, |v| v["conditions"] = json!("Other"));
    rejects(&value, |v| v["supervision_text"] = json!("Other"));
    rejects(&value, |v| v["validity"]["statement"] = json!("Other"));
    rejects(&value, |v| {
        v["validity"]["start"]["reason"] = json!("Other")
    });
    rejects(&value, |v| {
        v["validity"]["end"] = json!({"precision":"unknown","reason":"E"})
    });
    let other = with_times(unknown("Different"), None);
    assert!(values(&other.canonical_bytes(), &view(&value)).is_err());
}

#[test]
fn matching_bytes_and_projection_cannot_admit_a_domain_invalid_time_or_order() {
    let value = with_times(
        known(Declared::date("2026-10-04".parse().unwrap(), None).unwrap()),
        Some(known(
            Declared::date("2026-10-05".parse().unwrap(), None).unwrap(),
        )),
    );
    let bytes = value.canonical_bytes();
    assert_eq!(bytes.len(), 45);
    assert_eq!(bytes[37], 5);
    let mut reversed = bytes.clone();
    reversed[37] = 3;
    let mut projection = view(&value);
    projection["validity"]["end"]["day"] = json!(3);
    assert!(values(&reversed, &projection).is_err());

    let mut impossible = bytes;
    impossible[23] = 2;
    impossible[24] = 29;
    let mut projection = view(&value);
    projection["validity"]["start"]["month"] = json!(2);
    projection["validity"]["start"]["day"] = json!(29);
    assert!(values(&impossible, &projection).is_err());
}
