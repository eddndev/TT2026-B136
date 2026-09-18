use super::deadline_evaluation_input_encoding_support::*;
use application::{
    deadline_evaluations::{
        deadline_evaluation_input_bytes, decode_deadline_evaluation_input, DeadlineConditionAnswer,
    },
    deadline_inputs::DeadlineCalendarRef,
};
use domain::{
    deadline_triggers::TriggerSourceRef,
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

fn ordinary(source: &[u8], temporal: &[u8], calendar: &[u8], quantity: &[u8]) -> Vec<u8> {
    packet(
        source,
        temporal,
        calendar,
        quantity,
        &applicability(b"s", b"l", &[1, 1], &[1, 0], &[0; 4]),
    )
}

#[test]
fn independent_notification_vector_preserves_selection_quantity_and_full_declarations() {
    let mut input = minimal();
    input.selection.source = FactDeclaration::Known(TriggerSourceRef::Notification {
        id: NotificationId::from_uuid(Uuid::from_u128(2)),
        revision: FactRevision::new(3).unwrap(),
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(4)),
            revision: FactRevision::new(5).unwrap(),
        },
    });
    let mut temporal = qualified(
        DeclaredProceduralTime::second(
            "2028-02-29".parse().unwrap(),
            10,
            23,
            0,
            Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
        )
        .unwrap(),
    );
    temporal.statement = FactText::new("Start").unwrap();
    temporal.locator = FactLabel::new("P2").unwrap();
    input.selection.qualification = Some(temporal);
    input.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::from_uuid(Uuid::from_u128(6)),
        revision: JudicialCalendarRevision::new(7).unwrap(),
    });
    input.ordered_quantity = NonZeroU32::new(9);
    input.qualification.statement = FactText::new("Applies").unwrap();
    input.qualification.locator = FactLabel::new("L1").unwrap();
    input.qualification.scope_applies = FactDeclaration::Unknown(FactText::new("Unsure").unwrap());
    input.qualification.conditions = vec![
        DeadlineConditionAnswer {
            id: Uuid::from_u128(9),
            applies: FactDeclaration::Known(true),
            locator: FactLabel::new("L2").unwrap(),
        },
        DeadlineConditionAnswer {
            id: Uuid::nil(),
            applies: FactDeclaration::Unknown(FactText::new("Open").unwrap()),
            locator: FactLabel::new("L3").unwrap(),
        },
    ];
    let source = [
        vec![1, 1],
        Uuid::from_u128(2).as_bytes().to_vec(),
        3u32.to_be_bytes().to_vec(),
        Uuid::from_u128(4).as_bytes().to_vec(),
        5u32.to_be_bytes().to_vec(),
    ]
    .concat();
    let temporal = [
        vec![1, 1, 3, 0x07, 0xec, 2, 29, 10, 23, 0, 1],
        (-21600i32).to_be_bytes().to_vec(),
        text(b"Start"),
        text(b"P2"),
    ]
    .concat();
    let calendar = [
        vec![1],
        Uuid::from_u128(6).as_bytes().to_vec(),
        7u32.to_be_bytes().to_vec(),
    ]
    .concat();
    let quantity = [vec![1], 9u32.to_be_bytes().to_vec()].concat();
    let conditions = [
        2u32.to_be_bytes().to_vec(),
        Uuid::from_u128(9).as_bytes().to_vec(),
        vec![1, 1],
        text(b"L2"),
        Uuid::nil().as_bytes().to_vec(),
        unknown(b"Open"),
        text(b"L3"),
    ]
    .concat();
    let expected = packet(
        &source,
        &temporal,
        &calendar,
        &quantity,
        &applicability(b"Applies", b"L1", &unknown(b"Unsure"), &[1, 0], &conditions),
    );
    assert_eq!(deadline_evaluation_input_bytes(&input).unwrap(), expected);
    assert_eq!(decode_deadline_evaluation_input(&expected).unwrap(), input);
}

#[test]
fn every_truncation_wrong_header_and_trailing_bytes_are_rejected() {
    let mut input = minimal();
    input.selection.qualification = Some(qualified(DeclaredProceduralTime::unknown()));
    for good in [minimum_bytes(), roundtrip(&input)] {
        for end in 0..good.len() {
            rejected(&good[..end]);
        }
        let mut trailing = good.clone();
        trailing.push(0);
        rejected(&trailing);
        let mut wrong = good;
        wrong[0] = b'X';
        rejected(&wrong);
    }
}

#[test]
fn invalid_option_declaration_boolean_family_tags_and_zero_counters_are_rejected() {
    for index in [21, 27, 28, 29, 40, 41, 42, 43] {
        let mut bytes = minimum_bytes();
        bytes[index] = 2;
        rejected(&bytes);
    }
    for count in [17u32, u32::MAX] {
        let mut bytes = minimum_bytes();
        bytes[44..48].copy_from_slice(&count.to_be_bytes());
        rejected(&bytes);
    }
    rejected(&ordinary(&[1, 3], &[0], &[0], &[0]));
    let bad_revision = [vec![1, 0], vec![0; 16], vec![0; 4]].concat();
    rejected(&ordinary(&bad_revision, &[0], &[0], &[0]));
    let bad_calendar = [vec![1], vec![0; 16], vec![0; 4]].concat();
    rejected(&ordinary(&unknown(b"r"), &[0], &bad_calendar, &[0]));
    rejected(&ordinary(&unknown(b"r"), &[0], &[0], &[1, 0, 0, 0, 0]));
}

#[test]
fn decoder_rejects_duplicate_condition_ids_without_silently_sorting_or_deduplicating() {
    let answer = [vec![0; 16], vec![1, 1], text(b"l")].concat();
    let conditions = [2u32.to_be_bytes().to_vec(), answer.clone(), answer].concat();
    rejected(&packet(
        &unknown(b"r"),
        &[0],
        &[0],
        &[0],
        &applicability(b"s", b"l", &[1, 1], &[1, 0], &conditions),
    ));
}

#[test]
fn malformed_utf8_scalar_bounds_controls_and_alternative_normalizations_are_rejected() {
    for reason in [
        vec![],
        vec![0xff],
        b" r".to_vec(),
        b"r ".to_vec(),
        b"a\r\nb".to_vec(),
        b"a\0b".to_vec(),
        vec![b'x'; 1001],
        vec![b'x'; 4001],
    ] {
        rejected(&ordinary(&unknown(&reason), &[0], &[0], &[0]));
    }
    let huge = [vec![0], u32::MAX.to_be_bytes().to_vec()].concat();
    rejected(&ordinary(&huge, &[0], &[0], &[0]));
    for (statement, locator) in [
        (b" s".as_slice(), b"l".as_slice()),
        (b"s", b"l\nx"),
        (b"s", b"l "),
        (b"s", b""),
    ] {
        rejected(&packet(
            &unknown(b"r"),
            &[0],
            &[0],
            &[0],
            &applicability(statement, locator, &[1, 1], &[1, 0], &[0; 4]),
        ));
    }
    rejected(&packet(
        &unknown(b"r"),
        &[0],
        &[0],
        &[0],
        &applicability(b"s", b"l", &[1, 2], &[1, 0], &[0; 4]),
    ));
    rejected(&packet(
        &unknown(b"r"),
        &[0],
        &[0],
        &[0],
        &applicability(b"s", b"l", &unknown(b" r"), &[1, 0], &[0; 4]),
    ));
}

#[test]
fn malformed_temporal_qualification_never_invents_a_valid_time() {
    let invalid_times = vec![
        vec![4],
        vec![1, 0x07, 0xe9, 2, 29, 0],
        vec![2, 0x07, 0xec, 2, 29, 24, 0, 0],
        vec![3, 0x07, 0xec, 2, 29, 10, 23, 60, 0],
        vec![1, 0, 0, 1, 1, 0],
        vec![1, 0x07, 0xec, 2, 29, 2],
        vec![0, 1, 0, 0, 0, 0],
        [vec![1, 0x07, 0xec, 2, 29, 1], 1i32.to_be_bytes().to_vec()].concat(),
        [vec![1, 0, 1, 1, 1, 1], 50400i32.to_be_bytes().to_vec()].concat(),
    ];
    for at in invalid_times {
        let temporal = [vec![1, 1], at, text(b"s"), text(b"l")].concat();
        rejected(&ordinary(&unknown(b"r"), &temporal, &[0], &[0]));
    }
    let bad_purpose = [vec![1, 2, 0], text(b"s"), text(b"l")].concat();
    rejected(&ordinary(&unknown(b"r"), &bad_purpose, &[0], &[0]));
}
