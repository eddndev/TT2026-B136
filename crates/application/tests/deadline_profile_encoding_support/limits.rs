use super::deadline_profile_encoding_support::*;
use application::deadline_profiles::*;
use domain::{
    deadline_arithmetic::{DayBasis, DayInclusion, FinalDayPolicy},
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    deadline_triggers::{QualifiedTriggerPurpose, TriggerFamily, TriggerRequirement},
    judicial_calendars::{
        JudicialCalendarSource, JudicialCalendarSourceInput, JudicialCalendarValues,
    },
    procedural_time::DeclaredProceduralTime,
};
use time::{Time, UtcOffset};

fn largest_calendar() -> JudicialCalendarValues {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../domain/tests/fixtures/judicial_calendar_vectors.json"
    ))
    .unwrap();
    let entry = fixture
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["bytes"] == 191910)
        .unwrap();
    let (pairs, remaining) = entry["hex"].as_str().unwrap().as_bytes().as_chunks::<2>();
    assert!(remaining.is_empty());
    let bytes = pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    JudicialCalendarValues::from_canonical_bytes(&bytes).unwrap()
}

#[test]
fn sixteen_maximum_calendars_and_large_unicode_fields_fit_the_derived_upper_bound() {
    let mut values = input();
    let astral = "\u{1f680}";
    let calendar = largest_calendar();
    values.title = label(&astral.repeat(200));
    values.description = text(&astral.repeat(1000));
    values.scope = DeadlineProfileScope::Global(calendar.scope().clone());
    let url = format!("https://a.aa/{}", "x".repeat(2035));
    assert_eq!(url.len(), 2048);
    values.references = (0..16)
        .map(|value| {
            JudicialCalendarSource::new(JudicialCalendarSourceInput {
                id: id(value),
                title: &astral.repeat(200),
                issuer: &astral.repeat(200),
                official_url: &url,
                published_on: Some(date("2026-01-01")),
                consulted_on: date("2026-02-01"),
                locator: &astral.repeat(512),
            })
            .unwrap()
        })
        .collect();
    values.trigger = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        family: TriggerFamily::Notification,
    };
    values.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::Days {
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        },
        maximum: Some(quantity(1)),
    };
    values.completion = DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            Time::from_hms(17, 25, 1).unwrap(),
            UtcOffset::UTC,
            date("2026-01-01"),
            date("2026-12-31"),
            label(&astral.repeat(200)),
            id(0),
        )
        .unwrap(),
    );
    values.conditions = (0..16)
        .map(|value| DeadlineProfileCondition {
            id: id(value),
            statement: text(&astral.repeat(1000)),
            reference_ids: (0..16).rev().map(id).collect(),
        })
        .collect();
    let original = values.examples[0].clone();
    values.examples = (0..16)
        .map(|value| {
            let mut example = original.clone();
            example.id = id(value);
            example.anchor = DeclaredProceduralTime::second(
                date("2026-01-06"),
                12,
                25,
                59,
                Some(UtcOffset::UTC),
            )
            .unwrap();
            example.ordered_quantity = Some(quantity(if value == 0 { 1 } else { 2 }));
            example.calendar = Some(calendar.clone());
            example.reference_ids = (0..16).map(id).collect();
            example.locator = label(&astral.repeat(200));
            example
        })
        .collect();
    recalculate(&mut values);
    let bytes = roundtrip(values);
    assert!(bytes.len() > 3_000_000);
    assert!(bytes.len() <= 3_261_697);
}

#[test]
fn embedded_jcal1_length_prefix_and_internal_canonical_form_are_strict() {
    let mut values = input();
    values.examples[0].calendar = Some(calendar(
        domain::judicial_calendars::JudicialCalendarClassification::Countable,
    ));
    let bytes = roundtrip(values);
    let start = bytes.windows(5).position(|part| part == b"JCAL1").unwrap();
    let length = u32::from_be_bytes(bytes[start - 4..start].try_into().unwrap()) as usize;
    for invalid_length in [0_u32, 98, 191911, u32::MAX] {
        let mut invalid = bytes.clone();
        invalid[start - 4..start].copy_from_slice(&invalid_length.to_be_bytes());
        assert!(decode_deadline_profile_definition(&invalid).is_err());
    }
    let mut invalid_header = bytes.clone();
    invalid_header[start] = b'X';
    assert!(decode_deadline_profile_definition(&invalid_header).is_err());
    let mut trailing_calendar = bytes.clone();
    trailing_calendar.insert(start + length, 0);
    trailing_calendar[start - 4..start].copy_from_slice(&((length + 1) as u32).to_be_bytes());
    assert!(decode_deadline_profile_definition(&trailing_calendar).is_err());
    let title_length = u32::from_be_bytes(bytes[start + 5..start + 9].try_into().unwrap()) as usize;
    let codes = start + 9 + title_length + 2;
    assert_eq!(&bytes[codes..codes + 2], &[1, 32]);
    let mut reordered = bytes;
    reordered.swap(codes, codes + 1);
    assert!(decode_deadline_profile_definition(&reordered).is_err());
}
