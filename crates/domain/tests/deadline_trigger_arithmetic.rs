mod deadline_days_support;
mod deadline_trigger_qualification_support;
mod procedural_fact_support;

use deadline_days_support::{calendar, date, quantity};
use deadline_trigger_qualification_support::{case_id, qualification, ResolutionFixture};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{
        ArithmeticBlock, ArithmeticOutcome, ArithmeticRule, ArithmeticTraceStep, DayBasis,
        DayInclusion, FinalDayPolicy,
    },
    deadline_triggers::*,
    judicial_calendars::JudicialCalendarClassification::{Countable, Excluded},
    procedural_facts::{FactDeclaration, FactText},
    procedural_time::DeclaredProceduralTime as Declared,
};
use time::{macros::datetime, UtcOffset};
use uuid::Uuid;

fn countable_day() -> ArithmeticRule {
    ArithmeticRule::Days {
        quantity: quantity(1),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    }
}

#[test]
fn coordinator_uses_exact_qualified_anchor_and_preserves_original_offset() {
    let fixture = ResolutionFixture::new(Declared::unknown());
    let at = Declared::second(
        date("2026-12-31"),
        23,
        45,
        17,
        Some(UtcOffset::from_hms(2, 30, 0).unwrap()),
    )
    .unwrap();
    let selection = fixture.selection(Some(qualification(
        QualifiedTriggerPurpose::OrderedPeriodStart,
        at,
    )));
    let requirement = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        family: TriggerFamily::Resolution,
    };
    let rule = ArithmeticRule::ElapsedHours {
        quantity: quantity(10),
    };
    let result = evaluate_triggered_arithmetic(
        requirement,
        &selection,
        Some(fixture.material()),
        rule,
        None,
    )
    .unwrap();
    assert_eq!(result.rule(), rule);
    assert_eq!(result.trigger().requirement(), requirement);
    assert_eq!(result.trigger().selection(), &selection);
    assert_eq!(
        result.trigger().outcome(),
        &TriggerOutcome::Extracted { at }
    );
    let arithmetic = result.arithmetic().unwrap();
    assert_eq!(arithmetic.anchor(), at);
    assert_eq!(arithmetic.rule(), rule);
    assert_eq!(
        arithmetic.outcome(),
        &ArithmeticOutcome::InstantCandidate {
            instant: datetime!(2027-01-01 07:15:17 UTC),
        }
    );
    assert_eq!(
        arithmetic.trace(),
        &[ArithmeticTraceStep::ElapsedHours {
            start: datetime!(2026-12-31 21:15:17 UTC),
            quantity: quantity(10),
            candidate: Some(datetime!(2027-01-01 07:15:17 UTC)),
        }]
    );
}

#[test]
fn extracted_unknown_is_an_arithmetic_unknown_anchor_not_a_missing_source() {
    let fixture = ResolutionFixture::new(Declared::unknown());
    let selection = fixture.selection(None);
    let result = evaluate_triggered_arithmetic(
        TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        &selection,
        Some(fixture.material()),
        ArithmeticRule::ElapsedHours {
            quantity: quantity(2),
        },
        None,
    )
    .unwrap();
    assert!(result.trigger().source().is_some());
    assert_eq!(
        result.trigger().outcome(),
        &TriggerOutcome::Extracted {
            at: Declared::unknown()
        }
    );
    let arithmetic = result.arithmetic().unwrap();
    assert_eq!(arithmetic.anchor(), Declared::unknown());
    assert_eq!(
        arithmetic.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor)
    );
    assert!(arithmetic.trace().is_empty());
}

#[test]
fn a_blocked_source_does_not_run_even_an_arithmetic_rule_missing_its_calendar() {
    let selection = TriggerSelection {
        case_id: case_id(),
        source: FactDeclaration::Unknown(
            FactText::new("The exact source is not identified").unwrap(),
        ),
        qualification: None,
    };
    let rule = countable_day();
    let result = evaluate_triggered_arithmetic(
        TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        &selection,
        None,
        rule,
        None,
    )
    .unwrap();
    assert_eq!(result.rule(), rule);
    assert_eq!(result.trigger().selection(), &selection);
    assert_eq!(
        result.trigger().outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
    assert!(result.trigger().source().is_none());
    assert!(result.arithmetic().is_none());
}

#[test]
fn calendar_changes_arithmetic_without_changing_the_extracted_source() {
    let at = Declared::date(date("2026-03-02"), None).unwrap();
    let fixture = ResolutionFixture::new(at);
    let selection = fixture.selection(None);
    let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
    let rule = countable_day();
    let every_day = calendar("2026-03-01", "2026-03-31", [Countable; 7], vec![]);
    let skip_monday = calendar(
        "2026-03-01",
        "2026-03-31",
        [
            Excluded, Countable, Countable, Countable, Countable, Countable, Countable,
        ],
        vec![],
    );
    let first = evaluate_triggered_arithmetic(
        requirement,
        &selection,
        Some(fixture.material()),
        rule,
        Some(&every_day),
    )
    .unwrap();
    let second = evaluate_triggered_arithmetic(
        requirement,
        &selection,
        Some(fixture.material()),
        rule,
        Some(&skip_monday),
    )
    .unwrap();
    let absent = evaluate_triggered_arithmetic(
        requirement,
        &selection,
        Some(fixture.material()),
        rule,
        None,
    )
    .unwrap();
    assert_eq!(first.trigger(), second.trigger());
    assert_eq!(first.trigger(), absent.trigger());
    assert_eq!(first.arithmetic().unwrap().anchor(), at);
    assert_eq!(second.arithmetic().unwrap().anchor(), at);
    assert_eq!(
        first.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-03-02"),
        }
    );
    assert_eq!(
        second.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-03-03"),
        }
    );
    assert_eq!(
        absent.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingCalendar)
    );
}

#[test]
fn integrity_failure_propagates_before_family_or_arithmetic_blocks() {
    let fixture = ResolutionFixture::new(Declared::unknown());
    let mut selection = fixture.selection(None);
    selection.case_id = CaseId::from_uuid(Uuid::from_u128(2));
    let error = evaluate_triggered_arithmetic(
        TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::HearingEnd,
            family: TriggerFamily::HearingResult,
        },
        &selection,
        Some(fixture.material()),
        countable_day(),
        None,
    )
    .unwrap_err();
    assert_eq!(error, TriggerIntegrityError::CaseMismatch);
}

#[test]
fn monthly_arithmetic_retains_the_exact_anchor_and_missing_homolog_block() {
    let at = Declared::date(date("2024-01-31"), Some(UtcOffset::UTC)).unwrap();
    let fixture = ResolutionFixture::new(at);
    let selection = fixture.selection(None);
    let rule = ArithmeticRule::CivilMonths {
        quantity: quantity(1),
        final_day: FinalDayPolicy::Preserve,
    };
    let result = evaluate_triggered_arithmetic(
        TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        &selection,
        Some(fixture.material()),
        rule,
        None,
    )
    .unwrap();
    assert_eq!(
        result.trigger().outcome(),
        &TriggerOutcome::Extracted { at }
    );
    let arithmetic = result.arithmetic().unwrap();
    assert_eq!(arithmetic.anchor(), at);
    assert_eq!(arithmetic.rule(), rule);
    assert_eq!(
        arithmetic.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: 2024,
            month: 2,
            requested_day: 31
        })
    );
    assert_eq!(
        arithmetic.trace(),
        &[ArithmeticTraceStep::CivilMonths {
            anchor: date("2024-01-31"),
            quantity: quantity(1),
            target_year: 2024,
            target_month: 2,
            requested_day: 31,
            candidate: None,
        }]
    );
}
