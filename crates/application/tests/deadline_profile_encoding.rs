mod deadline_profile_encoding_support;

use application::deadline_profiles::*;
use deadline_profile_encoding_support::*;
use domain::{
    deadline_arithmetic::{
        ArithmeticOutcome, ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy,
    },
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    deadline_triggers::{QualifiedTriggerPurpose, TriggerFamily, TriggerField, TriggerRequirement},
    judicial_calendars::{
        JudicialCalendarClassification, JudicialCalendarSource, JudicialCalendarSourceInput,
    },
    procedural_time::DeclaredProceduralTime,
};
use std::collections::HashSet;
use time::{Duration, Time, UtcOffset};

#[test]
fn minimum_case_scope_and_global_scope_preserve_every_declared_field() {
    let case = roundtrip(input());
    assert_eq!(case.len(), 202);
    let mut global = input();
    global.title = label("Perfil \u{e1}");
    global.description = text("Primera linea\nTexto e\u{301} \u{1f680}");
    global.scope = DeadlineProfileScope::Global(scope());
    assert_ne!(roundtrip(global), case);
}

#[test]
fn public_references_keep_dates_urls_fragments_locators_and_declared_order() {
    let mut values = input();
    values.references = vec![
        JudicialCalendarSource::new(JudicialCalendarSourceInput {
            id: id(9),
            title: "Publicaci\u{f3}n",
            issuer: "Issuer",
            official_url: "https://example.org/a%20b?edition=2#p9",
            published_on: Some(date("2026-01-01")),
            consulted_on: date("2026-02-01"),
            locator: "p9",
        })
        .unwrap(),
        source(0),
    ];
    values.conditions[0].reference_ids = vec![id(9), id(0)];
    values.examples[0].reference_ids = vec![id(0), id(9)];
    let first = roundtrip(values.clone());
    values.references.reverse();
    assert_ne!(roundtrip(values.clone()), first);
    values.references.reverse();
    values.conditions[0].reference_ids.reverse();
    assert_ne!(roundtrip(values.clone()), first);
    values.conditions[0].reference_ids.reverse();
    values.examples[0].reference_ids.reverse();
    assert_ne!(roundtrip(values), first);
}

#[test]
fn sixteen_references_conditions_examples_and_links_keep_ids_and_order() {
    let mut values = input();
    values.references = (0..16).rev().map(source).collect();
    let original_condition = values.conditions[0].clone();
    let original_example = values.examples[0].clone();
    values.conditions = (0..16)
        .rev()
        .map(|value| {
            let mut condition = original_condition.clone();
            condition.id = id(value);
            condition.reference_ids = (0..16).rev().map(id).collect();
            condition
        })
        .collect();
    values.examples = (0..16)
        .map(|value| {
            let mut example = original_example.clone();
            example.id = id(value);
            example.reference_ids = (0..16).map(id).collect();
            example
        })
        .collect();
    let first = roundtrip(values.clone());
    values.conditions.reverse();
    assert_ne!(roundtrip(values.clone()), first);
    values.conditions.reverse();
    values.examples.reverse();
    assert_ne!(roundtrip(values), first);
}

#[test]
fn all_requirements_are_preserved_without_inventing_source_applicability() {
    let mut requirements = [
        TriggerField::ResolutionIssuedAt,
        TriggerField::NotificationPracticedAt,
        TriggerField::NotificationReceivedAt,
        TriggerField::NotificationStatedEffectAt,
        TriggerField::HearingSessionEventTime,
    ]
    .into_iter()
    .map(TriggerRequirement::SourceField)
    .collect::<Vec<_>>();
    for purpose in [
        QualifiedTriggerPurpose::HearingEnd,
        QualifiedTriggerPurpose::OrderedPeriodStart,
    ] {
        for family in [
            TriggerFamily::Resolution,
            TriggerFamily::Notification,
            TriggerFamily::HearingResult,
        ] {
            requirements.push(TriggerRequirement::Qualified { purpose, family });
        }
    }
    let mut encodings = HashSet::new();
    for trigger in requirements {
        let mut values = input();
        values.trigger = trigger;
        assert!(encodings.insert(roundtrip(values)));
    }
    assert_eq!(encodings.len(), 11);
}

#[test]
fn fixed_and_ordered_units_policies_and_optional_maxima_are_distinct() {
    let mut units = vec![OrderedDeadlineUnit::ElapsedHours];
    for final_day in [FinalDayPolicy::Preserve, FinalDayPolicy::NextCountable] {
        units.push(OrderedDeadlineUnit::CivilMonths { final_day });
        for inclusion in [DayInclusion::OnAnchor, DayInclusion::AfterAnchor] {
            for basis in [DayBasis::Natural, DayBasis::CalendarCountable] {
                units.push(OrderedDeadlineUnit::Days {
                    inclusion,
                    basis,
                    final_day,
                });
            }
        }
    }
    let mut encodings = HashSet::new();
    for unit in units {
        for maximum in [None, Some(quantity(2)), Some(quantity(u32::MAX))] {
            let template = DeadlineRuleTemplate::Ordered { unit, maximum };
            for ordered in [false, true] {
                let mut values = input();
                values.template = if ordered {
                    template
                } else {
                    DeadlineRuleTemplate::Fixed(template.instantiate(Some(quantity(1))).unwrap())
                };
                values.completion = if unit == OrderedDeadlineUnit::ElapsedHours {
                    DeadlineCompletionPolicy::ArithmeticInstant
                } else {
                    DeadlineCompletionPolicy::CivilCandidateOnly
                };
                values.examples[0].anchor = DeclaredProceduralTime::second(
                    date("2026-01-06"),
                    10,
                    20,
                    30,
                    Some(UtcOffset::UTC),
                )
                .unwrap();
                values.examples[0].ordered_quantity = ordered.then(|| quantity(1));
                values.examples[0].calendar =
                    Some(calendar(JudicialCalendarClassification::Countable));
                recalculate(&mut values);
                let encoded = roundtrip(values);
                if ordered || maximum.is_none() {
                    assert!(encodings.insert(encoded));
                }
            }
        }
    }
    assert_eq!(encodings.len(), 44);
}

#[test]
fn cutoff_retains_its_own_offset_channel_reference_and_inclusive_coverage() {
    let mut values = input();
    values.examples[0].anchor = DeclaredProceduralTime::second(
        date("1970-01-01"),
        9,
        0,
        0,
        Some(UtcOffset::from_hms(9, 0, 0).unwrap()),
    )
    .unwrap();
    let base = roundtrip(values.clone());
    values.completion = DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            Time::from_hms(17, 25, 1).unwrap(),
            UtcOffset::from_hms(-6, 0, 0).unwrap(),
            date("2027-01-01"),
            date("2027-12-31"),
            label("Declared filing channel"),
            id(0),
        )
        .unwrap(),
    );
    assert_ne!(roundtrip(values), base);
}

#[test]
fn all_declared_anchor_precisions_and_offset_presence_survive_corpus_encoding() {
    let mut values = input();
    let original = values.examples[0].clone();
    values.examples.clear();
    let mut anchors = vec![DeclaredProceduralTime::unknown()];
    for offset in [
        None,
        Some(UtcOffset::UTC),
        Some(UtcOffset::from_hms(-14, 0, 0).unwrap()),
    ] {
        anchors.extend([
            DeclaredProceduralTime::date(date("1970-01-01"), offset).unwrap(),
            DeclaredProceduralTime::minute(date("1970-01-01"), 10, 15, offset).unwrap(),
            DeclaredProceduralTime::second(date("1970-01-01"), 10, 15, 0, offset).unwrap(),
            DeclaredProceduralTime::second(date("1970-01-01"), 10, 15, 59, offset).unwrap(),
        ]);
    }
    for (index, anchor) in anchors.into_iter().enumerate() {
        let mut example = original.clone();
        example.id = id(index as u128);
        example.anchor = anchor;
        values.examples.push(example);
    }
    recalculate(&mut values);
    roundtrip(values);
}

#[test]
fn expected_instant_keeps_original_offset_even_when_eq_only_compares_instant() {
    let mut values = input();
    values.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours {
        quantity: quantity(1),
    });
    values.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    values.examples[0].anchor =
        DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    recalculate(&mut values);
    let utc = roundtrip(values.clone());
    let DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate { instant }) =
        values.examples[0].expected
    else {
        unreachable!()
    };
    for seconds in [1, -86399, 86399] {
        values.examples[0].expected =
            DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
                instant: instant.to_offset(UtcOffset::from_whole_seconds(seconds).unwrap()),
            });
        assert_ne!(roundtrip(values.clone()), utc);
    }
    values.examples[0].anchor =
        DeclaredProceduralTime::second(date("0001-01-01"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    recalculate(&mut values);
    let DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate { instant }) =
        values.examples[0].expected
    else {
        unreachable!()
    };
    let previous_year = instant.to_offset(UtcOffset::from_hms(-2, 0, 0).unwrap());
    assert_eq!(previous_year.year(), 0);
    values.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: previous_year,
        });
    roundtrip(values);
}

#[test]
fn expected_fractional_instant_is_rejected_when_the_whole_second_example_does_not_reproduce_it() {
    let mut values = input();
    values.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours {
        quantity: quantity(1),
    });
    values.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    values.examples[0].anchor =
        DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    recalculate(&mut values);
    let DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate { instant }) =
        values.examples[0].expected
    else {
        unreachable!()
    };
    values.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: instant + Duration::nanoseconds(1),
        });
    assert!(matches!(
        DeadlineProfileDefinition::new(values),
        Err(DeadlineProfileError::ExampleMismatch(_))
    ));
}

#[test]
fn embedded_calendars_remain_exact_even_when_unused_by_the_rule() {
    let mut values = input();
    values.examples[0].calendar = Some(calendar(JudicialCalendarClassification::Excluded));
    let first = roundtrip(values.clone());
    values.examples[0].calendar = Some(calendar(JudicialCalendarClassification::Unresolved));
    assert_ne!(roundtrip(values.clone()), first);
    values.examples[0].calendar = None;
    assert_ne!(roundtrip(values), first);
}

#[path = "deadline_profile_encoding_support/limits.rs"]
mod limits;
#[path = "deadline_profile_encoding_support/wire.rs"]
mod wire;

#[path = "deadline_profile_encoding_support/blocks.rs"]
mod blocks;
