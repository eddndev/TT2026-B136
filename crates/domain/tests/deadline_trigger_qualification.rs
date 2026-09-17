mod deadline_trigger_qualification_support;
mod procedural_fact_support;

use deadline_trigger_qualification_support::{qualification, HearingFixture, ResolutionFixture};
use domain::{
    deadline_triggers::*,
    procedural_facts::{FactLabel, FactText},
    procedural_time::DeclaredProceduralTime as Declared,
};
use time::UtcOffset;

#[test]
fn concluded_session_and_event_time_do_not_supply_missing_hearing_end() {
    let fixture = HearingFixture::concluded();
    let selection = fixture.selection(None);
    let requirement = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::HearingEnd,
        family: TriggerFamily::HearingResult,
    };
    let result = extract_trigger_time(requirement, &selection, Some(fixture.material())).unwrap();
    assert_eq!(result.requirement(), requirement);
    assert_eq!(result.selection(), &selection);
    assert!(result.source().is_some());
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::MissingQualification {
            purpose: QualifiedTriggerPurpose::HearingEnd
        })
    );
}

#[test]
fn a_different_qualified_purpose_blocks_without_falling_back_to_event_time() {
    let fixture = HearingFixture::concluded();
    let selection = fixture.selection(Some(qualification(
        QualifiedTriggerPurpose::OrderedPeriodStart,
        Declared::unknown(),
    )));
    let result = extract_trigger_time(
        TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::HearingEnd,
            family: TriggerFamily::HearingResult,
        },
        &selection,
        Some(fixture.material()),
    )
    .unwrap();
    assert!(result.source().is_some());
    assert_eq!(result.selection(), &selection);
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::QualificationMismatch {
            expected: QualifiedTriggerPurpose::HearingEnd,
            actual: QualifiedTriggerPurpose::OrderedPeriodStart,
        })
    );
}

#[test]
fn source_field_does_not_ignore_an_unexpected_qualification() {
    let fixture = ResolutionFixture::new(Declared::unknown());
    let selection = fixture.selection(Some(qualification(
        QualifiedTriggerPurpose::OrderedPeriodStart,
        Declared::unknown(),
    )));
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        &selection,
        Some(fixture.material()),
    )
    .unwrap();
    assert!(result.source().is_some());
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnexpectedQualification)
    );
}

#[test]
fn an_explicit_incompatible_family_blocks_before_missing_qualification() {
    let fixture = ResolutionFixture::new(Declared::unknown());
    let selection = fixture.selection(None);
    let result = extract_trigger_time(
        TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::HearingEnd,
            family: TriggerFamily::HearingResult,
        },
        &selection,
        Some(fixture.material()),
    )
    .unwrap();
    assert!(result.source().is_some());
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::IncompatibleFamily {
            expected: TriggerFamily::HearingResult,
            actual: TriggerFamily::Resolution,
        })
    );
}

#[test]
fn qualified_time_statement_and_locator_are_preserved_without_shared_mutation() {
    let fixture = HearingFixture::concluded();
    let date = "2026-03-03".parse().unwrap();
    let times = [
        Declared::unknown(),
        Declared::date(date, None).unwrap(),
        Declared::minute(date, 11, 42, Some(UtcOffset::UTC)).unwrap(),
        Declared::second(
            date,
            11,
            42,
            0,
            Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
        )
        .unwrap(),
    ];
    for at in times {
        let mut selection =
            fixture.selection(Some(qualification(QualifiedTriggerPurpose::HearingEnd, at)));
        let expected = selection.clone();
        let result = extract_trigger_time(
            TriggerRequirement::Qualified {
                purpose: QualifiedTriggerPurpose::HearingEnd,
                family: TriggerFamily::HearingResult,
            },
            &selection,
            Some(fixture.material()),
        )
        .unwrap();
        let selected = selection.qualification.as_mut().unwrap();
        selected.at = Declared::unknown();
        selected.statement = FactText::new("A later changed statement").unwrap();
        selected.locator = FactLabel::new("A later locator").unwrap();
        assert_eq!(result.selection(), &expected);
        assert_eq!(result.outcome(), &TriggerOutcome::Extracted { at });
        assert_eq!(
            result.source().unwrap().reference,
            TriggerSourceRef::HearingResult(fixture.reference())
        );
    }
}

#[test]
fn a_purpose_label_does_not_override_the_explicit_source_family() {
    let fixture = ResolutionFixture::new(Declared::unknown());
    let at = Declared::date("2026-04-07".parse().unwrap(), None).unwrap();
    let selection = fixture.selection(Some(qualification(QualifiedTriggerPurpose::HearingEnd, at)));
    let result = extract_trigger_time(
        TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::HearingEnd,
            family: TriggerFamily::Resolution,
        },
        &selection,
        Some(fixture.material()),
    )
    .unwrap();
    assert_eq!(result.outcome(), &TriggerOutcome::Extracted { at });
    assert_eq!(
        result.source().unwrap().reference,
        TriggerSourceRef::Resolution(fixture.reference())
    );
}
