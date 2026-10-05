#[path = "measure_decision_codec_support/invalid.rs"]
mod invalid;
#[path = "measure_decision_codec_support/mod.rs"]
mod support;

use domain::precautionary_measures::*;
use domain::procedural_time::DeclaredProceduralTime;
use infrastructure::measure_decision_codec::{
    decision_values, decision_view, measure_values, measure_view, outcome, outcome_view,
};
use serde_json::json;
use support::*;
use time::{Month, UtcOffset};

#[test]
fn decision_measure_and_all_effects_round_trip_without_source_or_authority_claims() {
    let declared = decision();
    assert_eq!(
        decision_values(&declared.canonical_bytes(), &decision_view(&declared)).unwrap(),
        declared
    );
    let terms = measure();
    assert_eq!(
        measure_values(&terms.canonical_bytes(), &measure_view(&terms)).unwrap(),
        terms
    );
    let effects = changes();
    assert_eq!(
        outcome(&effects.canonical_bytes(), &outcome_view(&effects)).unwrap(),
        effects
    );
}

#[test]
fn scalar_decision_and_measure_bytes_match_independent_existing_frames() {
    let terms = unhex(concat!(
        "4d454153310000000000000000000000000000000100000002",
        "1111111111111111111111111111111111111111111111111111111111111111",
        "0d0000000343c3a9000000124d56414c310107ea0a040000000000015600",
        "0000000000000000000000000000000003000000040000000153"
    ));
    assert_eq!(terms.len(), 113);
    assert_eq!(
        measure_values(&terms, &measure_view(&measure())).unwrap(),
        measure()
    );
    let declared = unhex(concat!(
        "4d4456414c31000000034ac3a90307ea0a0401020301ffffaba000000000014a",
        "0000000000000000000000000000000500000006",
        "2222222222222222222222222222222222222222222222222222222222222222",
        "000000014c"
    ));
    assert_eq!(declared.len(), 89);
    assert_eq!(
        decision_values(&declared, &decision_view(&decision())).unwrap(),
        decision()
    );
}

#[test]
fn confirm_and_no_change_use_independent_mefx1_bytes() {
    let confirm = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Confirm {
            previous: reference(10),
        },
    ]))
    .unwrap();
    let bytes = unhex(concat!(
        "4d45465831000000000101",
        "0000000000000000000000000000000a00000002",
        "3333333333333333333333333333333333333333333333333333333333333333"
    ));
    assert_eq!(bytes.len(), 63);
    assert_eq!(outcome(&bytes, &outcome_view(&confirm)).unwrap(), confirm);
    let no_change =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(note("N")))
            .unwrap();
    assert_eq!(
        outcome(b"MEFX1\x01\0\0\0\x01N", &outcome_view(&no_change)).unwrap(),
        no_change
    );
}

#[test]
fn projections_use_only_explicit_domain_components_and_exact_reference_keys() {
    assert_eq!(
        decision_view(&decision()),
        json!({
            "authority":"J\u{e9}",
            "declared_at":{"precision":"second","year":2026,"month":10,"day":4,
                "hour":1,"minute":2,"second":3,"offset_seconds":-21600},
            "justification":"J",
            "support":{"document_id":"00000000-0000-0000-0000-000000000005",
                "version":6,"digest":"22".repeat(32)},
            "locator":"L"
        })
    );
    assert_eq!(
        measure_view(&measure()),
        json!({
            "subject":{"id":"00000000-0000-0000-0000-000000000001",
                "revision":2,"digest":"11".repeat(32)},
            "kind":"pretrial_detention","conditions":"C\u{e9}",
            "validity":{"start":{"precision":"date","year":2026,"month":10,
                "day":4,"offset_seconds":null},"statement":"V","end":null},
            "supervision":{"kind":"known",
                "participant":{"id":"00000000-0000-0000-0000-000000000003","revision":4},
                "statement":"S"}
        })
    );
}

#[test]
fn every_declared_precision_and_optional_offset_round_trips_in_both_value_families() {
    let mut times = vec![time()];
    for offset in [
        None,
        Some(UtcOffset::UTC),
        Some(UtcOffset::from_hms(-14, 0, 0).unwrap()),
        Some(UtcOffset::from_hms(14, 0, 0).unwrap()),
    ] {
        let day = date(2024, Month::February, 29);
        times.push(known_time(
            DeclaredProceduralTime::date(day, offset).unwrap(),
        ));
        times.push(known_time(
            DeclaredProceduralTime::minute(day, 23, 59, offset).unwrap(),
        ));
        times.push(known_time(
            DeclaredProceduralTime::second(day, 23, 59, 59, offset).unwrap(),
        ));
    }
    for declared_at in times {
        let mut input = decision_input();
        input.declared_at = declared_at.clone();
        let decision = MeasureDecisionValues::new(input);
        assert_eq!(
            decision_values(&decision.canonical_bytes(), &decision_view(&decision)).unwrap(),
            decision
        );
        let mut input = measure_input();
        input.validity = MeasureValidity::new(
            declared_at.clone(),
            note("Same declared endpoint"),
            Some(declared_at),
        )
        .unwrap();
        let measure = MeasureValues::new(input);
        assert_eq!(
            measure_values(&measure.canonical_bytes(), &measure_view(&measure)).unwrap(),
            measure
        );
    }
}

#[test]
fn boundary_years_and_unknown_time_preserve_absence_without_synthetic_instants() {
    for day in [date(1, Month::January, 1), date(9999, Month::December, 31)] {
        for offset in [None, Some(UtcOffset::UTC)] {
            let mut input = decision_input();
            input.declared_at = known_time(DeclaredProceduralTime::date(day, offset).unwrap());
            let value = MeasureDecisionValues::new(input);
            assert_eq!(
                decision_values(&value.canonical_bytes(), &decision_view(&value)).unwrap(),
                value
            );
        }
    }
    let mut input = decision_input();
    input.declared_at = time();
    let value = MeasureDecisionValues::new(input);
    assert_eq!(
        decision_view(&value)["declared_at"],
        json!({"precision":"unknown","reason":"U"})
    );
    assert_eq!(
        decision_values(&value.canonical_bytes(), &decision_view(&value)).unwrap(),
        value
    );
}

#[test]
fn all_fourteen_declared_measure_kinds_round_trip() {
    for kind in [
        MeasureKind::PeriodicAppearance,
        MeasureKind::FinancialGuarantee,
        MeasureKind::AssetSeizure,
        MeasureKind::AccountFreeze,
        MeasureKind::TravelRestriction,
        MeasureKind::CustodyOrInstitution,
        MeasureKind::PlaceRestriction,
        MeasureKind::ContactRestriction,
        MeasureKind::HomeSeparation,
        MeasureKind::PublicOfficeSuspension,
        MeasureKind::ProfessionalSuspension,
        MeasureKind::ElectronicMonitoring,
        MeasureKind::HomeConfinement,
        MeasureKind::PretrialDetention,
    ] {
        let mut input = measure_input();
        input.kind = kind;
        let value = MeasureValues::new(input);
        assert_eq!(measure_view(&value)["kind"], kind.as_str());
        assert_eq!(
            measure_values(&value.canonical_bytes(), &measure_view(&value)).unwrap(),
            value
        );
    }
}

#[test]
fn unknown_supervision_and_end_presence_remain_explicit() {
    for end in [None, Some(time())] {
        let mut input = measure_input();
        input.supervision = MeasureSupervision::Unknown {
            reason: note("Not stated"),
        };
        input.validity =
            MeasureValidity::new(time(), note("Declared validity"), end.clone()).unwrap();
        let value = MeasureValues::new(input);
        let projection = measure_view(&value);
        assert_eq!(
            projection["supervision"],
            json!({"kind":"unknown","reason":"Not stated"})
        );
        assert_eq!(projection["validity"]["end"].is_null(), end.is_none());
        assert_eq!(
            measure_values(&value.canonical_bytes(), &projection).unwrap(),
            value
        );
    }
}

#[test]
fn separate_substitution_groups_preserve_sorted_exact_membership() {
    let value = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Substitute {
            predecessors: vec![reference(21), reference(20)],
            successors: vec![proposal(31), proposal(30)],
        },
        MeasureEffect::Substitute {
            predecessors: vec![reference(2), reference(1)],
            successors: vec![proposal(11), proposal(10)],
        },
        MeasureEffect::Impose(proposal(50)),
    ]))
    .unwrap();
    let projection = outcome_view(&value);
    assert_eq!(
        projection["effects"][0]["predecessors"][0]["id"],
        reference(1).id().to_string()
    );
    assert_eq!(
        projection["effects"][0]["successors"][0]["id"],
        proposal(10).id.to_string()
    );
    assert_eq!(
        projection["effects"][1]["predecessors"][0]["id"],
        reference(20).id().to_string()
    );
    assert_eq!(
        outcome(&value.canonical_bytes(), &projection).unwrap(),
        value
    );
}

#[test]
fn zero_change_retains_its_statement_and_has_no_fabricated_effect_array() {
    let value = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(note(
        "No declared change",
    )))
    .unwrap();
    assert_eq!(
        outcome_view(&value),
        json!({"kind":"no_measure_change","statement":"No declared change"})
    );
    assert_eq!(
        outcome(&value.canonical_bytes(), &outcome_view(&value)).unwrap(),
        value
    );
}

#[test]
fn escaped_ascii_and_mixed_unicode_text_remain_exact_after_json_transport() {
    for text in ["A\n\\\"".repeat(250), "A\u{e9}\u{1f642}\nB\\\"".to_owned()] {
        let mut input = measure_input();
        input.conditions = note(&text);
        let value = MeasureValues::new(input);
        let encoded = serde_json::to_vec(&measure_view(&value)).unwrap();
        let transported = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            measure_values(&value.canonical_bytes(), &transported).unwrap(),
            value
        );
        assert_eq!(value.conditions().as_str(), text);
    }
}
