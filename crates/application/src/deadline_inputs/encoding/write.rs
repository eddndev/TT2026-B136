use super::{DeadlineInputRequest, PREFIX};
use crate::deadline_inputs::DeadlineCalendarRef;
use domain::{
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_triggers::{
        QualifiedTriggerPurpose, TriggerFamily, TriggerField, TriggerRequirement, TriggerSelection,
        TriggerSourceRef,
    },
    procedural_facts::{FactDeclaration, FactResolutionRef},
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};

pub(super) fn encode(request: &DeadlineInputRequest) -> Vec<u8> {
    let mut bytes = PREFIX.to_vec();
    selection(&mut bytes, &request.trigger);
    requirement(&mut bytes, request.requirement);
    rule(&mut bytes, request.rule);
    calendar(&mut bytes, request.calendar);
    bytes
}
pub(crate) fn selection(bytes: &mut Vec<u8>, value: &TriggerSelection) {
    bytes.extend_from_slice(value.case_id.as_uuid().as_bytes());
    match &value.source {
        FactDeclaration::Unknown(reason) => {
            bytes.push(0);
            text(bytes, reason.as_str());
        }
        FactDeclaration::Known(reference) => {
            bytes.push(1);
            source(bytes, *reference);
        }
    }
    bytes.push(u8::from(value.qualification.is_some()));
    if let Some(value) = &value.qualification {
        bytes.push(purpose(value.purpose));
        declared_time(bytes, value.at);
        text(bytes, value.statement.as_str());
        text(bytes, value.locator.as_str());
    }
}
pub(crate) fn calendar(bytes: &mut Vec<u8>, value: Option<DeadlineCalendarRef>) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        bytes.extend_from_slice(value.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&value.revision.get().to_be_bytes());
    }
}
pub(crate) fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
fn resolution(bytes: &mut Vec<u8>, value: FactResolutionRef) {
    bytes.extend_from_slice(value.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision.get().to_be_bytes());
}
fn source(bytes: &mut Vec<u8>, reference: TriggerSourceRef) {
    bytes.push(family(reference.family()));
    match reference {
        TriggerSourceRef::Resolution(value) => resolution(bytes, value),
        TriggerSourceRef::Notification {
            id,
            revision,
            resolution: parent,
        } => {
            bytes.extend_from_slice(id.as_uuid().as_bytes());
            bytes.extend_from_slice(&revision.get().to_be_bytes());
            resolution(bytes, parent);
        }
        TriggerSourceRef::HearingResult(value) => {
            bytes.extend_from_slice(value.hearing_id.as_uuid().as_bytes());
            bytes.extend_from_slice(value.result_id.as_uuid().as_bytes());
            bytes.extend_from_slice(&value.revision.get().to_be_bytes());
            bytes.push(u8::from(value.agreement_id.is_some()));
            if let Some(id) = value.agreement_id {
                bytes.extend_from_slice(id.as_uuid().as_bytes());
            }
        }
    }
}
fn family(value: TriggerFamily) -> u8 {
    match value {
        TriggerFamily::Resolution => 0,
        TriggerFamily::Notification => 1,
        TriggerFamily::HearingResult => 2,
    }
}
fn purpose(value: QualifiedTriggerPurpose) -> u8 {
    match value {
        QualifiedTriggerPurpose::HearingEnd => 0,
        QualifiedTriggerPurpose::OrderedPeriodStart => 1,
    }
}
pub(crate) fn requirement(bytes: &mut Vec<u8>, value: TriggerRequirement) {
    match value {
        TriggerRequirement::SourceField(field) => {
            bytes.push(0);
            bytes.push(match field {
                TriggerField::ResolutionIssuedAt => 0,
                TriggerField::NotificationPracticedAt => 1,
                TriggerField::NotificationReceivedAt => 2,
                TriggerField::NotificationStatedEffectAt => 3,
                TriggerField::HearingSessionEventTime => 4,
            });
        }
        TriggerRequirement::Qualified {
            purpose: p,
            family: f,
        } => {
            bytes.extend_from_slice(&[1, purpose(p), family(f)]);
        }
    }
}
fn final_day(value: FinalDayPolicy) -> u8 {
    match value {
        FinalDayPolicy::Preserve => 0,
        FinalDayPolicy::NextCountable => 1,
    }
}
pub(crate) fn rule(bytes: &mut Vec<u8>, value: ArithmeticRule) {
    match value {
        ArithmeticRule::Days {
            quantity,
            inclusion,
            basis,
            final_day: policy,
        } => {
            bytes.push(0);
            bytes.extend_from_slice(&quantity.get().to_be_bytes());
            bytes.push(match inclusion {
                DayInclusion::OnAnchor => 0,
                DayInclusion::AfterAnchor => 1,
            });
            bytes.push(match basis {
                DayBasis::Natural => 0,
                DayBasis::CalendarCountable => 1,
            });
            bytes.push(final_day(policy));
        }
        ArithmeticRule::CivilMonths {
            quantity,
            final_day: policy,
        } => {
            bytes.push(1);
            bytes.extend_from_slice(&quantity.get().to_be_bytes());
            bytes.push(final_day(policy));
        }
        ArithmeticRule::ElapsedHours { quantity } => {
            bytes.push(2);
            bytes.extend_from_slice(&quantity.get().to_be_bytes());
        }
    }
}
pub(crate) fn declared_time(bytes: &mut Vec<u8>, value: DeclaredProceduralTime) {
    bytes.push(match value.precision() {
        DeclaredProceduralPrecision::Unknown => 0,
        DeclaredProceduralPrecision::Date => 1,
        DeclaredProceduralPrecision::Minute => 2,
        DeclaredProceduralPrecision::Second => 3,
    });
    if let Some(date) = value.local_date() {
        let date = date.date();
        bytes.extend_from_slice(&(date.year() as u16).to_be_bytes());
        bytes.extend_from_slice(&[date.month() as u8, date.day()]);
        if let Some(hour) = value.local_hour() {
            bytes.push(hour);
        }
        if let Some(minute) = value.local_minute() {
            bytes.push(minute);
        }
        if let Some(second) = value.local_second() {
            bytes.push(second);
        }
        bytes.push(u8::from(value.offset().is_some()));
        if let Some(offset) = value.offset() {
            bytes.extend_from_slice(&offset.whole_seconds().to_be_bytes());
        }
    }
}
