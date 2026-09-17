use application::deadline_inputs::{
    deadline_input_request_bytes, decode_deadline_input_request, DeadlineInputRequest,
};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_triggers::{
        QualifiedTriggerPurpose, QualifiedTriggerTime, TriggerField, TriggerRequirement,
        TriggerSelection, TriggerSourceRef,
    },
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
    procedural_facts::{
        FactDeclaration, FactHearingRef, FactLabel, FactResolutionRef, FactRevision, FactText,
        NotificationId, ResolutionId,
    },
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use uuid::Uuid;

pub fn unknown() -> DeadlineInputRequest {
    DeadlineInputRequest {
        trigger: TriggerSelection {
            case_id: CaseId::from_uuid(Uuid::from_u128(1)),
            source: FactDeclaration::Unknown(FactText::new("x").unwrap()),
            qualification: None,
        },
        requirement: TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        rule: ArithmeticRule::ElapsedHours {
            quantity: NonZeroU32::new(1).unwrap(),
        },
        calendar: None,
    }
}

pub fn references() -> Vec<TriggerSourceRef> {
    let parent = FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::from_u128(2)),
        revision: FactRevision::new(u32::MAX).unwrap(),
    };
    let mut result = vec![
        TriggerSourceRef::Resolution(parent),
        TriggerSourceRef::Notification {
            id: NotificationId::from_uuid(Uuid::from_u128(3)),
            revision: FactRevision::new(7).unwrap(),
            resolution: parent,
        },
    ];
    for agreement_id in [
        None,
        Some(HearingResultAgreementId::from_uuid(Uuid::nil())),
        Some(HearingResultAgreementId::from_uuid(Uuid::from_u128(6))),
    ] {
        result.push(TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: HearingId::from_uuid(Uuid::from_u128(4)),
            result_id: HearingResultId::from_uuid(Uuid::from_u128(5)),
            revision: HearingResultRevision::new(u32::MAX).unwrap(),
            agreement_id,
        }));
    }
    result
}

pub fn qualification(at: DeclaredProceduralTime) -> QualifiedTriggerTime {
    QualifiedTriggerTime {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        at,
        statement: FactText::new("Declared start\nSecond line").unwrap(),
        locator: FactLabel::new("Page 3, paragraph 2").unwrap(),
    }
}

pub fn rules() -> Vec<ArithmeticRule> {
    let mut result = Vec::new();
    for quantity in [1, u32::MAX] {
        let quantity = NonZeroU32::new(quantity).unwrap();
        for inclusion in [DayInclusion::OnAnchor, DayInclusion::AfterAnchor] {
            for basis in [DayBasis::Natural, DayBasis::CalendarCountable] {
                for final_day in [FinalDayPolicy::Preserve, FinalDayPolicy::NextCountable] {
                    result.push(ArithmeticRule::Days {
                        quantity,
                        inclusion,
                        basis,
                        final_day,
                    });
                }
            }
        }
        for final_day in [FinalDayPolicy::Preserve, FinalDayPolicy::NextCountable] {
            result.push(ArithmeticRule::CivilMonths {
                quantity,
                final_day,
            });
        }
        result.push(ArithmeticRule::ElapsedHours { quantity });
    }
    result
}

pub fn roundtrip(request: &DeadlineInputRequest) -> Vec<u8> {
    let bytes = deadline_input_request_bytes(request);
    assert_eq!(bytes.get(..5), Some(b"DINP1".as_slice()));
    let decoded = decode_deadline_input_request(&bytes).unwrap();
    assert_eq!(&decoded, request);
    assert_eq!(deadline_input_request_bytes(&decoded), bytes);
    bytes
}
