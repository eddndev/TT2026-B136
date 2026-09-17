mod deadline_input_encoding_support;

use application::deadline_inputs::{DeadlineCalendarRef, DeadlineInputRequest};
use deadline_input_encoding_support::{qualification, references, roundtrip, rules, unknown};
use domain::{
    cases::CaseId,
    deadline_triggers::{
        QualifiedTriggerPurpose, TriggerFamily, TriggerField, TriggerRequirement, TriggerSourceRef,
    },
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::{
        FactDeclaration, FactHearingRef, FactLabel, FactResolutionRef, FactRevision, FactText,
        NotificationId, ResolutionId,
    },
    procedural_time::DeclaredProceduralTime,
};
use std::collections::HashSet;
use time::UtcOffset;
use uuid::Uuid;

#[test]
fn every_source_family_preserves_exact_ids_revisions_and_parent_or_agreement() {
    let mut encodings = HashSet::new();
    encodings.insert(roundtrip(&unknown()));
    for reference in references() {
        let mut request = unknown();
        request.trigger.source = FactDeclaration::Known(reference);
        assert!(encodings.insert(roundtrip(&request)));
    }
    assert_eq!(encodings.len(), 6);
}

#[test]
fn nil_ids_and_maximum_revisions_remain_valid_exact_references() {
    let parent = FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::nil()),
        revision: FactRevision::new(u32::MAX).unwrap(),
    };
    let selections = [
        TriggerSourceRef::Resolution(parent),
        TriggerSourceRef::Notification {
            id: NotificationId::from_uuid(Uuid::nil()),
            revision: FactRevision::new(u32::MAX).unwrap(),
            resolution: parent,
        },
        TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: HearingId::from_uuid(Uuid::nil()),
            result_id: HearingResultId::from_uuid(Uuid::nil()),
            revision: HearingResultRevision::new(u32::MAX).unwrap(),
            agreement_id: Some(HearingResultAgreementId::from_uuid(Uuid::nil())),
        }),
    ];
    for reference in selections {
        let mut request = unknown();
        request.trigger.case_id = CaseId::from_uuid(Uuid::nil());
        request.trigger.source = FactDeclaration::Known(reference);
        request.calendar = Some(DeadlineCalendarRef {
            id: JudicialCalendarId::from_uuid(Uuid::nil()),
            revision: JudicialCalendarRevision::new(u32::MAX).unwrap(),
        });
        roundtrip(&request);
    }
}

#[test]
fn every_requirement_keeps_field_or_qualified_purpose_and_family() {
    let mut requirements = vec![];
    for field in [
        TriggerField::ResolutionIssuedAt,
        TriggerField::NotificationPracticedAt,
        TriggerField::NotificationReceivedAt,
        TriggerField::NotificationStatedEffectAt,
        TriggerField::HearingSessionEventTime,
    ] {
        requirements.push(TriggerRequirement::SourceField(field));
    }
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
    for requirement in requirements {
        let mut request = unknown();
        request.requirement = requirement;
        assert!(encodings.insert(roundtrip(&request)));
    }
    assert_eq!(encodings.len(), 11);
}

#[test]
fn days_months_hours_and_each_mathematical_policy_are_distinct() {
    let mut encodings = HashSet::new();
    for rule in rules() {
        let mut request = unknown();
        request.rule = rule;
        assert!(encodings.insert(roundtrip(&request)));
    }
    assert_eq!(encodings.len(), 22);
}

#[test]
fn unknown_date_minute_second_and_absent_utc_offsets_never_collapse() {
    let date = "2028-02-29".parse().unwrap();
    let mut declarations = vec![DeclaredProceduralTime::unknown()];
    for offset in [
        None,
        Some(UtcOffset::UTC),
        Some(UtcOffset::from_hms(-14, 0, 0).unwrap()),
        Some(UtcOffset::from_hms(14, 0, 0).unwrap()),
    ] {
        declarations.push(DeclaredProceduralTime::date(date, offset).unwrap());
        declarations.push(DeclaredProceduralTime::minute(date, 10, 23, offset).unwrap());
        declarations.push(DeclaredProceduralTime::second(date, 10, 23, 0, offset).unwrap());
        declarations.push(DeclaredProceduralTime::second(date, 10, 23, 59, offset).unwrap());
    }
    let mut encodings = HashSet::new();
    let mut request = unknown();
    encodings.insert(roundtrip(&request));
    for at in declarations {
        request.trigger.qualification = Some(qualification(at));
        assert!(encodings.insert(roundtrip(&request)));
    }
    assert_eq!(encodings.len(), 18);
}

#[test]
fn local_and_utc_year_boundaries_roundtrip_without_replacing_declared_precision() {
    let first = "0001-01-01".parse().unwrap();
    let last = "9999-12-31".parse().unwrap();
    let plus = Some(UtcOffset::from_hms(14, 0, 0).unwrap());
    let minus = Some(UtcOffset::from_hms(-14, 0, 0).unwrap());
    for at in [
        DeclaredProceduralTime::date(first, None).unwrap(),
        DeclaredProceduralTime::date(last, Some(UtcOffset::UTC)).unwrap(),
        DeclaredProceduralTime::minute(first, 14, 0, plus).unwrap(),
        DeclaredProceduralTime::second(first, 14, 0, 0, plus).unwrap(),
        DeclaredProceduralTime::minute(last, 9, 59, minus).unwrap(),
        DeclaredProceduralTime::second(last, 9, 59, 59, minus).unwrap(),
    ] {
        let mut request = unknown();
        request.trigger.qualification = Some(qualification(at));
        roundtrip(&request);
    }
}

#[test]
fn structural_encoding_preserves_requests_that_semantically_block_extraction() {
    let mut request = unknown();
    request.trigger.qualification = Some(qualification(DeclaredProceduralTime::unknown()));
    roundtrip(&request);
    request.trigger.source = FactDeclaration::Known(references()[0]);
    request.requirement = TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime);
    roundtrip(&request);
    request.requirement = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::HearingEnd,
        family: TriggerFamily::Resolution,
    };
    roundtrip(&request);
}

#[test]
fn unicode_scalars_multiline_text_and_calendar_presence_are_preserved() {
    let mut request = unknown();
    request.trigger.source =
        FactDeclaration::Unknown(FactText::new("  Declaraci\u{f3}n\r\n\u{1f680}\r\n  ").unwrap());
    let mut declared = qualification(DeclaredProceduralTime::unknown());
    declared.statement = FactText::new("e\u{301}\n\u{e9}\nFinal").unwrap();
    declared.locator = FactLabel::new("\u{1f680} 7").unwrap();
    request.trigger.qualification = Some(declared);
    let absent = roundtrip(&request);
    request.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::from_uuid(Uuid::from_u128(8)),
        revision: JudicialCalendarRevision::new(9).unwrap(),
    });
    assert_ne!(roundtrip(&request), absent);
}

#[test]
fn bounded_unicode_fields_and_all_case_id_bits_survive_encoding() {
    let mut request: DeadlineInputRequest = unknown();
    request.trigger.case_id = CaseId::from_uuid(Uuid::from_u128(u128::MAX));
    request.trigger.source =
        FactDeclaration::Unknown(FactText::new(&"\u{1f680}".repeat(1000)).unwrap());
    let mut declared = qualification(DeclaredProceduralTime::unknown());
    declared.statement = FactText::new(&"\u{1f680}".repeat(1000)).unwrap();
    declared.locator = FactLabel::new(&"\u{1f680}".repeat(200)).unwrap();
    request.trigger.qualification = Some(declared);
    roundtrip(&request);
}

#[path = "deadline_input_encoding_support/wire.rs"]
mod wire;
