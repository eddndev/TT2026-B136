mod deadline_evaluation_input_encoding_support;
#[path = "deadline_evaluation_input_encoding_support/wire.rs"]
mod wire;

use application::{
    deadline_evaluations::{
        deadline_evaluation_input_bytes, decode_deadline_evaluation_input, DeadlineConditionAnswer,
    },
    deadline_inputs::DeadlineCalendarRef,
};
use deadline_evaluation_input_encoding_support::*;
use domain::{
    deadline_triggers::QualifiedTriggerPurpose,
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::{FactDeclaration, FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
};
use std::{collections::HashSet, num::NonZeroU32};
use time::UtcOffset;
use uuid::Uuid;

#[test]
fn independent_minimum_vector_fixes_all_tags_lengths_and_field_order() {
    let bytes = minimum_bytes();
    assert_eq!(bytes.len(), 48);
    assert_eq!(deadline_evaluation_input_bytes(&minimal()).unwrap(), bytes);
    assert_eq!(decode_deadline_evaluation_input(&bytes).unwrap(), minimal());
}

#[test]
fn exact_source_families_parent_and_nil_agreement_do_not_collapse() {
    let mut input = minimal();
    input.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::from_uuid(Uuid::nil()),
        revision: JudicialCalendarRevision::new(u32::MAX).unwrap(),
    });
    let mut encodings = HashSet::new();
    encodings.insert(roundtrip(&input));
    for source in sources() {
        input.selection.source = FactDeclaration::Known(source);
        assert!(encodings.insert(roundtrip(&input)));
    }
    assert_eq!(encodings.len(), 5);
}

#[test]
fn absent_ordered_quantity_is_not_replaced_with_one_or_a_maximum() {
    let mut input = minimal();
    let absent = roundtrip(&input);
    input.ordered_quantity = NonZeroU32::new(1);
    let one = roundtrip(&input);
    input.ordered_quantity = NonZeroU32::new(u32::MAX);
    let maximum = roundtrip(&input);
    assert_ne!(absent, one);
    assert_ne!(one, maximum);
    assert_ne!(absent, maximum);
}

#[test]
fn three_state_declarations_and_condition_order_remain_explicit() {
    let declarations = [
        FactDeclaration::Known(false),
        FactDeclaration::Known(true),
        FactDeclaration::Unknown(FactText::new("Not established").unwrap()),
    ];
    let mut encodings = HashSet::new();
    for scope in &declarations {
        for incident in &declarations {
            let mut input = minimal();
            input.qualification.scope_applies = scope.clone();
            input.qualification.unresolved_incident = incident.clone();
            input.qualification.conditions = declarations
                .iter()
                .enumerate()
                .map(|(i, applies)| DeadlineConditionAnswer {
                    id: Uuid::from_u128(i as u128),
                    applies: applies.clone(),
                    locator: FactLabel::new("Separate condition").unwrap(),
                })
                .collect();
            assert!(encodings.insert(roundtrip(&input)));
            input.qualification.conditions.reverse();
            assert!(encodings.insert(roundtrip(&input)));
        }
    }
    assert_eq!(encodings.len(), 18);
}

#[test]
fn temporal_qualification_preserves_precision_seconds_offset_and_unknown_source() {
    let date = "2028-02-29".parse().unwrap();
    let mut times = vec![DeclaredProceduralTime::unknown()];
    for offset in [
        None,
        Some(UtcOffset::UTC),
        Some(UtcOffset::from_hms(-14, 0, 0).unwrap()),
    ] {
        times.push(DeclaredProceduralTime::date(date, offset).unwrap());
        times.push(DeclaredProceduralTime::minute(date, 10, 23, offset).unwrap());
        times.push(DeclaredProceduralTime::second(date, 10, 23, 0, offset).unwrap());
        times.push(DeclaredProceduralTime::second(date, 10, 23, 59, offset).unwrap());
    }
    let mut input = minimal();
    let mut encodings = HashSet::new();
    encodings.insert(roundtrip(&input));
    for purpose in [
        QualifiedTriggerPurpose::HearingEnd,
        QualifiedTriggerPurpose::OrderedPeriodStart,
    ] {
        for at in &times {
            let mut value = qualified(*at);
            value.purpose = purpose;
            input.selection.qualification = Some(value);
            assert!(encodings.insert(roundtrip(&input)));
        }
    }
    assert_eq!(encodings.len(), 27);
}

#[test]
fn full_unicode_bounds_and_sixteen_conditions_reach_the_wire_size_limit() {
    let mut input = minimal();
    let longest_text = FactText::new(&"\u{1f680}".repeat(1000)).unwrap();
    let longest_label = FactLabel::new(&"\u{1f680}".repeat(200)).unwrap();
    input.selection.source = FactDeclaration::Unknown(longest_text.clone());
    let mut temporal = qualified(
        DeclaredProceduralTime::second(
            "2028-02-29".parse().unwrap(),
            10,
            23,
            59,
            Some(UtcOffset::UTC),
        )
        .unwrap(),
    );
    temporal.statement = longest_text.clone();
    temporal.locator = longest_label.clone();
    input.selection.qualification = Some(temporal);
    input.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::from_uuid(Uuid::nil()),
        revision: JudicialCalendarRevision::new(u32::MAX).unwrap(),
    });
    input.ordered_quantity = NonZeroU32::new(u32::MAX);
    input.qualification.statement = longest_text.clone();
    input.qualification.locator = longest_label.clone();
    input.qualification.scope_applies = FactDeclaration::Unknown(longest_text.clone());
    input.qualification.unresolved_incident = FactDeclaration::Unknown(longest_text.clone());
    input.qualification.conditions = (0..16)
        .map(|i| DeadlineConditionAnswer {
            id: Uuid::from_u128(i),
            applies: FactDeclaration::Unknown(longest_text.clone()),
            locator: longest_label.clone(),
        })
        .collect();
    let mut bytes = roundtrip(&input);
    assert_eq!(bytes.len(), 98_897);
    bytes.push(0);
    rejected(&bytes);
}

#[test]
fn encoder_rejects_duplicate_ids_and_excess_count_independently_of_any_profile() {
    let mut input = minimal();
    let answer = DeadlineConditionAnswer {
        id: Uuid::nil(),
        applies: FactDeclaration::Known(true),
        locator: FactLabel::new("Condition").unwrap(),
    };
    input.qualification.conditions = vec![answer.clone(), answer.clone()];
    assert!(deadline_evaluation_input_bytes(&input).is_err());
    input.qualification.conditions = (0..17)
        .map(|i| DeadlineConditionAnswer {
            id: Uuid::from_u128(i),
            ..answer.clone()
        })
        .collect();
    assert!(deadline_evaluation_input_bytes(&input).is_err());
}

#[test]
fn constructors_normalize_once_without_losing_internal_unicode_or_line_breaks() {
    let mut input = minimal();
    input.qualification.statement = FactText::new("  \u{e9}\r\ne\u{301}\nEnd  ").unwrap();
    input.qualification.locator = FactLabel::new(" \u{1f680} 3 ").unwrap();
    input.qualification.scope_applies =
        FactDeclaration::Unknown(FactText::new("  Reason\r\nNext  ").unwrap());
    let bytes = roundtrip(&input);
    let decoded = decode_deadline_evaluation_input(&bytes).unwrap();
    assert_eq!(
        decoded.qualification.statement.as_str(),
        "\u{e9}\ne\u{301}\nEnd"
    );
    assert_eq!(decoded.qualification.locator.as_str(), "\u{1f680} 3");
}
