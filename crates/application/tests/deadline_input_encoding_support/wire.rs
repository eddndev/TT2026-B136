use super::deadline_input_encoding_support::{qualification, roundtrip, unknown};
use application::deadline_inputs::{
    deadline_input_request_bytes, decode_deadline_input_request, DeadlineCalendarRef,
};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_triggers::{
        QualifiedTriggerPurpose, TriggerFamily, TriggerRequirement, TriggerSourceRef,
    },
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::{
        FactDeclaration, FactLabel, FactResolutionRef, FactRevision, FactText, NotificationId,
        ResolutionId,
    },
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use time::UtcOffset;
use uuid::Uuid;

fn text(value: &[u8]) -> Vec<u8> {
    let mut result = (value.len() as u32).to_be_bytes().to_vec();
    result.extend_from_slice(value);
    result
}
fn source(reason: &[u8]) -> Vec<u8> {
    [vec![0], text(reason)].concat()
}
fn packet(source: &[u8], qualified: &[u8], required: &[u8], rule: &[u8], cal: &[u8]) -> Vec<u8> {
    [
        b"DINP1".as_slice(),
        Uuid::from_u128(1).as_bytes(),
        source,
        qualified,
        required,
        rule,
        cal,
    ]
    .concat()
}
fn plain(source: &[u8]) -> Vec<u8> {
    packet(source, &[0], &[0, 0], &[2, 0, 0, 0, 1], &[0])
}
fn qualified(time: &[u8], statement: &[u8], locator: &[u8]) -> Vec<u8> {
    [vec![1, 1], time.to_vec(), text(statement), text(locator)].concat()
}
fn time(precision: u8, year: u16, month: u8, day: u8, clock: &[u8], offset: &[u8]) -> Vec<u8> {
    [
        vec![precision],
        year.to_be_bytes().to_vec(),
        vec![month, day],
        clock.to_vec(),
        offset.to_vec(),
    ]
    .concat()
}
fn offset(seconds: i32) -> Vec<u8> {
    [vec![1], seconds.to_be_bytes().to_vec()].concat()
}
fn rejected(bytes: &[u8]) {
    assert!(
        decode_deadline_input_request(bytes).is_err(),
        "accepted {bytes:?}"
    );
}

#[test]
fn independent_minimum_vector_fixes_prefix_byte_lengths_tags_and_field_order() {
    let mut request = unknown();
    request.trigger.case_id = CaseId::from_uuid(Uuid::nil());
    let expected = [
        b"DINP1".as_slice(),
        &[0; 16],
        &[0, 0, 0, 0, 1, b'x', 0, 0, 0, 2, 0, 0, 0, 1, 0],
    ]
    .concat();
    assert_eq!(expected.len(), 36);
    assert_eq!(deadline_input_request_bytes(&request), expected);
    assert_eq!(decode_deadline_input_request(&expected).unwrap(), request);
    assert_eq!(
        decode_deadline_input_request(&plain(&source(b"x"))).unwrap(),
        unknown()
    );
}

#[test]
fn independent_notification_vector_preserves_parent_time_months_and_calendar() {
    let mut request = unknown();
    request.trigger.source = FactDeclaration::Known(TriggerSourceRef::Notification {
        id: NotificationId::from_uuid(Uuid::from_u128(2)),
        revision: FactRevision::new(3).unwrap(),
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(4)),
            revision: FactRevision::new(5).unwrap(),
        },
    });
    let at = DeclaredProceduralTime::second(
        "2028-02-29".parse().unwrap(),
        10,
        23,
        0,
        Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
    )
    .unwrap();
    let mut declaration = qualification(at);
    declaration.statement = FactText::new("Start").unwrap();
    declaration.locator = FactLabel::new("P2").unwrap();
    request.trigger.qualification = Some(declaration);
    request.requirement = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        family: TriggerFamily::Notification,
    };
    request.rule = ArithmeticRule::CivilMonths {
        quantity: NonZeroU32::new(6).unwrap(),
        final_day: FinalDayPolicy::NextCountable,
    };
    request.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::from_uuid(Uuid::from_u128(6)),
        revision: JudicialCalendarRevision::new(7).unwrap(),
    });
    let reference = [
        vec![1, 1],
        Uuid::from_u128(2).as_bytes().to_vec(),
        3_u32.to_be_bytes().to_vec(),
        Uuid::from_u128(4).as_bytes().to_vec(),
        5_u32.to_be_bytes().to_vec(),
    ]
    .concat();
    let clock = time(3, 2028, 2, 29, &[10, 23, 0], &offset(-21600));
    let cal = [
        vec![1],
        Uuid::from_u128(6).as_bytes().to_vec(),
        7_u32.to_be_bytes().to_vec(),
    ]
    .concat();
    let expected = packet(
        &reference,
        &qualified(&clock, b"Start", b"P2"),
        &[1, 1, 1],
        &[1, 0, 0, 0, 6, 1],
        &cal,
    );
    assert_eq!(deadline_input_request_bytes(&request), expected);
    assert_eq!(decode_deadline_input_request(&expected).unwrap(), request);
    for length in 0..expected.len() {
        rejected(&expected[..length]);
    }
    let mut trailing = expected;
    trailing.push(0);
    rejected(&trailing);
}

#[test]
fn largest_utf8_request_is_bounded_without_rejecting_unknown_qualification() {
    let mut request = unknown();
    request.trigger.source =
        FactDeclaration::Unknown(FactText::new(&"\u{1f680}".repeat(1000)).unwrap());
    let at = DeclaredProceduralTime::second(
        "2028-02-29".parse().unwrap(),
        10,
        23,
        59,
        Some(UtcOffset::UTC),
    )
    .unwrap();
    let mut declaration = qualification(at);
    declaration.statement = FactText::new(&"\u{1f680}".repeat(1000)).unwrap();
    declaration.locator = FactLabel::new(&"\u{1f680}".repeat(200)).unwrap();
    request.trigger.qualification = Some(declaration);
    request.requirement = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::HearingEnd,
        family: TriggerFamily::HearingResult,
    };
    request.rule = ArithmeticRule::Days {
        quantity: NonZeroU32::new(u32::MAX).unwrap(),
        inclusion: DayInclusion::AfterAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::NextCountable,
    };
    request.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::from_uuid(Uuid::nil()),
        revision: JudicialCalendarRevision::new(u32::MAX).unwrap(),
    });
    let mut bytes = roundtrip(&request);
    assert_eq!(bytes.len(), 8881);
    bytes.push(0);
    rejected(&bytes);
}

#[test]
fn invalid_prefix_and_all_discriminant_families_are_rejected() {
    let base = plain(&source(b"x"));
    for position in 0..5 {
        let mut wrong = base.clone();
        wrong[position] ^= 1;
        rejected(&wrong);
    }
    for value in [vec![2], vec![1, 3]] {
        rejected(&plain(&value));
    }
    for required in [vec![2], vec![0, 5], vec![1, 2, 0], vec![1, 0, 3]] {
        rejected(&packet(
            &source(b"x"),
            &[0],
            &required,
            &[2, 0, 0, 0, 1],
            &[0],
        ));
    }
    for rule in [
        vec![3, 0, 0, 0, 1],
        vec![0, 0, 0, 0, 1, 2, 0, 0],
        vec![0, 0, 0, 0, 1, 0, 2, 0],
        vec![0, 0, 0, 0, 1, 0, 0, 2],
        vec![1, 0, 0, 0, 1, 2],
    ] {
        rejected(&packet(&source(b"x"), &[0], &[0, 0], &rule, &[0]));
    }
    for qual in [vec![2], qualified(&[4], b"s", b"l")] {
        rejected(&packet(
            &source(b"x"),
            &qual,
            &[0, 0],
            &[2, 0, 0, 0, 1],
            &[0],
        ));
    }
    let mut qual = qualified(&[0], b"s", b"l");
    qual[1] = 2;
    rejected(&packet(
        &source(b"x"),
        &qual,
        &[0, 0],
        &[2, 0, 0, 0, 1],
        &[0],
    ));
    rejected(&packet(
        &source(b"x"),
        &[0],
        &[0, 0],
        &[2, 0, 0, 0, 1],
        &[2],
    ));
    let mut hearing = vec![1, 2];
    hearing.extend_from_slice(&[0; 32]);
    hearing.extend_from_slice(&1_u32.to_be_bytes());
    hearing.push(2);
    rejected(&plain(&hearing));
}

#[test]
fn zero_revisions_and_quantities_are_rejected_without_wrapping() {
    for (family, lengths, revisions) in
        [(0, 16, vec![18]), (1, 36, vec![18, 38]), (2, 32, vec![34])]
    {
        let mut reference = vec![1, family];
        reference.extend(vec![0; lengths]);
        reference.extend_from_slice(&1_u32.to_be_bytes());
        if family == 1 {
            reference[18..22].copy_from_slice(&1_u32.to_be_bytes());
        }
        if family == 2 {
            reference.push(0);
        }
        assert!(decode_deadline_input_request(&plain(&reference)).is_ok());
        for start in revisions {
            let mut invalid = reference.clone();
            invalid[start..start + 4].fill(0);
            rejected(&plain(&invalid));
        }
    }
    for rule in [vec![0; 8], vec![1, 0, 0, 0, 0, 0], vec![2, 0, 0, 0, 0]] {
        rejected(&packet(&source(b"x"), &[0], &[0, 0], &rule, &[0]));
    }
    let mut cal = vec![1];
    cal.extend_from_slice(&[0; 20]);
    rejected(&packet(
        &source(b"x"),
        &[0],
        &[0, 0],
        &[2, 0, 0, 0, 1],
        &cal,
    ));
}

#[test]
fn noncanonical_invalid_or_oversized_text_is_rejected_in_each_text_position() {
    for invalid in [
        b"".to_vec(),
        b" x".to_vec(),
        b"x ".to_vec(),
        b"a\r\nb".to_vec(),
        vec![255],
        b"a\0b".to_vec(),
        b"a\tb".to_vec(),
        vec![b'x'; 1001],
    ] {
        rejected(&plain(&source(&invalid)));
        let qual = qualified(&[0], &invalid, b"locator");
        rejected(&packet(
            &source(b"x"),
            &qual,
            &[0, 0],
            &[2, 0, 0, 0, 1],
            &[0],
        ));
    }
    for invalid in [
        b"".to_vec(),
        b" x".to_vec(),
        b"x ".to_vec(),
        b"a\nb".to_vec(),
        vec![255],
        vec![b'x'; 201],
    ] {
        let qual = qualified(&[0], b"statement", &invalid);
        rejected(&packet(
            &source(b"x"),
            &qual,
            &[0, 0],
            &[2, 0, 0, 0, 1],
            &[0],
        ));
    }
    for length in [2_u32, u32::MAX] {
        let malformed = [vec![0], length.to_be_bytes().to_vec(), vec![b'x']].concat();
        rejected(&plain(&malformed));
    }
}

#[test]
fn temporal_components_offset_and_complete_interval_bounds_are_validated() {
    let invalid = vec![
        time(1, 0, 1, 1, &[], &[0]),
        time(1, 10000, 1, 1, &[], &[0]),
        time(1, 2026, 0, 1, &[], &[0]),
        time(1, 2026, 13, 1, &[], &[0]),
        time(1, 2026, 1, 0, &[], &[0]),
        time(1, 2026, 2, 29, &[], &[0]),
        time(2, 2026, 1, 1, &[24, 0], &[0]),
        time(2, 2026, 1, 1, &[10, 60], &[0]),
        time(3, 2026, 1, 1, &[10, 0, 60], &[0]),
        time(1, 2026, 1, 1, &[], &[2]),
        time(1, 2026, 1, 1, &[], &offset(1)),
        time(1, 2026, 1, 1, &[], &offset(50460)),
        time(1, 2026, 1, 1, &[], &offset(-50460)),
        time(1, 1, 1, 1, &[], &offset(60)),
        time(1, 9999, 12, 31, &[], &offset(-60)),
        time(2, 1, 1, 1, &[0, 0], &offset(60)),
        time(2, 9999, 12, 31, &[23, 59], &offset(-60)),
        time(3, 1, 1, 1, &[0, 0, 0], &offset(60)),
    ];
    for declared in invalid {
        let qual = qualified(&declared, b"s", b"l");
        rejected(&packet(
            &source(b"x"),
            &qual,
            &[0, 0],
            &[2, 0, 0, 0, 1],
            &[0],
        ));
    }
}
