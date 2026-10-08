use super::*;

#[test]
fn temporal_variants_cannot_acquire_or_omit_declared_components() {
    let unknown = stored_decision(unknown_decision());
    unknown.rejects(|view| view["declared_at"]["year"] = json!(2025));
    unknown.rejects(|view| view["declared_at"]["offset_seconds"] = Value::Null);
    for declared in [
        Declared::date(date(2025, Month::January, 2), None).unwrap(),
        Declared::minute(date(2025, Month::January, 2), 3, 4, None).unwrap(),
        Declared::second(date(2025, Month::January, 2), 3, 4, 5, Some(UtcOffset::UTC)).unwrap(),
    ] {
        let mut input = decision_input();
        input.declared_at = known_time(declared);
        let stored = stored_decision(MeasureDecisionValues::new(input));
        stored.exact_keys(&["/declared_at"]);
        stored.rejects(|view| view["declared_at"]["reason"] = json!("Not stated"));
        stored.rejects(|view| view["declared_at"]["nanoseconds"] = json!(0));
        for bad in [json!(0), json!(10000), json!(1.5), json!("2025")] {
            stored.rejects(|view| view["declared_at"]["year"] = bad);
        }
        for bad in [1, -50_401, 50_401, i64::MAX] {
            stored.rejects(|view| view["declared_at"]["offset_seconds"] = json!(bad));
        }
    }
}

#[test]
fn invalid_calendar_components_and_contradictory_validity_are_rejected() {
    let stored = stored_measure(known_measure());
    for (field, bad) in [
        ("month", 13),
        ("day", 32),
        ("hour", 24),
        ("minute", 60),
        ("second", 60),
    ] {
        stored.rejects(|view| view["validity"]["start"][field] = json!(bad));
    }
    stored.rejects(|view| {
        view["validity"]["start"]["month"] = json!(2);
        view["validity"]["start"]["day"] = json!(29);
    });
    stored.rejects(|view| view["validity"]["end"]["day"] = json!(1));
    stored.rejects(|view| view["validity"]["end"] = json!({"precision":"unknown"}));
}
