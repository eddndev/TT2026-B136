use application::deadline_evaluations::{
    deadline_evaluation_input_bytes, decode_deadline_evaluation_input, DeadlineApplicability,
    DeadlineEvaluationInput,
};
use domain::{
    cases::CaseId,
    deadline_triggers::{
        QualifiedTriggerPurpose, QualifiedTriggerTime, TriggerSelection, TriggerSourceRef,
    },
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
    procedural_facts::{
        FactDeclaration, FactHearingRef, FactLabel, FactResolutionRef, FactRevision, FactText,
        NotificationId, ResolutionId,
    },
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;

pub fn minimal() -> DeadlineEvaluationInput {
    DeadlineEvaluationInput {
        selection: TriggerSelection {
            case_id: CaseId::from_uuid(Uuid::nil()),
            source: FactDeclaration::Unknown(FactText::new("r").unwrap()),
            qualification: None,
        },
        calendar: None,
        ordered_quantity: None,
        qualification: DeadlineApplicability {
            statement: FactText::new("s").unwrap(),
            locator: FactLabel::new("l").unwrap(),
            scope_applies: FactDeclaration::Known(true),
            unresolved_incident: FactDeclaration::Known(false),
            conditions: vec![],
        },
    }
}

pub fn qualified(at: DeclaredProceduralTime) -> QualifiedTriggerTime {
    QualifiedTriggerTime {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        at,
        statement: FactText::new("Declared start\nSecond line").unwrap(),
        locator: FactLabel::new("Page 4").unwrap(),
    }
}

pub fn sources() -> Vec<TriggerSourceRef> {
    let parent = FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::nil()),
        revision: FactRevision::new(u32::MAX).unwrap(),
    };
    let mut values = vec![
        TriggerSourceRef::Resolution(parent),
        TriggerSourceRef::Notification {
            id: NotificationId::from_uuid(Uuid::from_u128(2)),
            revision: FactRevision::new(3).unwrap(),
            resolution: parent,
        },
    ];
    for agreement_id in [None, Some(HearingResultAgreementId::from_uuid(Uuid::nil()))] {
        values.push(TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: HearingId::from_uuid(Uuid::from_u128(4)),
            result_id: HearingResultId::from_uuid(Uuid::from_u128(5)),
            revision: HearingResultRevision::new(u32::MAX).unwrap(),
            agreement_id,
        }));
    }
    values
}

pub fn roundtrip(input: &DeadlineEvaluationInput) -> Vec<u8> {
    let bytes = deadline_evaluation_input_bytes(input).unwrap();
    let decoded = decode_deadline_evaluation_input(&bytes).unwrap();
    assert_eq!(&decoded, input);
    assert_eq!(deadline_evaluation_input_bytes(&decoded).unwrap(), bytes);
    bytes
}

pub fn text(value: &[u8]) -> Vec<u8> {
    [(value.len() as u32).to_be_bytes().as_slice(), value].concat()
}

pub fn unknown(reason: &[u8]) -> Vec<u8> {
    [vec![0], text(reason)].concat()
}

pub fn applicability(
    statement: &[u8],
    locator: &[u8],
    scope: &[u8],
    incident: &[u8],
    conditions: &[u8],
) -> Vec<u8> {
    [
        text(statement),
        text(locator),
        scope.to_vec(),
        incident.to_vec(),
        conditions.to_vec(),
    ]
    .concat()
}

pub fn packet(
    source: &[u8],
    qualification: &[u8],
    calendar: &[u8],
    quantity: &[u8],
    applies: &[u8],
) -> Vec<u8> {
    [
        b"DEVI1".as_slice(),
        &[0; 16],
        source,
        qualification,
        calendar,
        quantity,
        applies,
    ]
    .concat()
}

pub fn minimum_bytes() -> Vec<u8> {
    [
        b"DEVI1".as_slice(),
        &[0; 16],
        &[
            0, 0, 0, 0, 1, b'r', 0, 0, 0, 0, 0, 0, 1, b's', 0, 0, 0, 1, b'l', 1, 1, 1, 0, 0, 0, 0,
            0,
        ],
    ]
    .concat()
}

pub fn rejected(bytes: &[u8]) {
    assert!(
        decode_deadline_evaluation_input(bytes).is_err(),
        "accepted {} bytes",
        bytes.len()
    );
}
