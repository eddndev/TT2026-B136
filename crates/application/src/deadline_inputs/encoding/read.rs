use super::{invalid, reader::Reader, DeadlineInputRequest};
use crate::{deadline_inputs::DeadlineCalendarRef, ApplicationError};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_triggers::{
        QualifiedTriggerPurpose, QualifiedTriggerTime, TriggerFamily, TriggerField,
        TriggerRequirement, TriggerSelection, TriggerSourceRef,
    },
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
    judicial_calendars::{CivilDate, JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::{
        FactDeclaration, FactHearingRef, FactLabel, FactResolutionRef, FactRevision, FactText,
        NotificationId, ResolutionId,
    },
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use time::{Date, Month, UtcOffset};

pub(super) fn decode(reader: &mut Reader<'_>) -> Result<DeadlineInputRequest, ApplicationError> {
    Ok(DeadlineInputRequest {
        trigger: selection(reader)?,
        requirement: requirement(reader)?,
        rule: rule(reader)?,
        calendar: calendar(reader)?,
    })
}
pub(crate) fn selection(reader: &mut Reader<'_>) -> Result<TriggerSelection, ApplicationError> {
    let case_id = CaseId::from_uuid(reader.uuid()?);
    let source = if reader.flag()? {
        FactDeclaration::Known(source(reader)?)
    } else {
        FactDeclaration::Unknown(fact_text(reader)?)
    };
    let qualification = if reader.flag()? {
        Some(QualifiedTriggerTime {
            purpose: purpose(reader)?,
            at: declared_time(reader)?,
            statement: fact_text(reader)?,
            locator: FactLabel::new(reader.text(800)?).map_err(invalid)?,
        })
    } else {
        None
    };
    Ok(TriggerSelection {
        case_id,
        source,
        qualification,
    })
}
pub(crate) fn calendar(
    reader: &mut Reader<'_>,
) -> Result<Option<DeadlineCalendarRef>, ApplicationError> {
    Ok(if reader.flag()? {
        Some(DeadlineCalendarRef {
            id: JudicialCalendarId::from_uuid(reader.uuid()?),
            revision: JudicialCalendarRevision::new(reader.u32()?).map_err(invalid)?,
        })
    } else {
        None
    })
}
fn fact_text(reader: &mut Reader<'_>) -> Result<FactText, ApplicationError> {
    FactText::new(reader.text(4000)?).map_err(invalid)
}
fn resolution(reader: &mut Reader<'_>) -> Result<FactResolutionRef, ApplicationError> {
    Ok(FactResolutionRef {
        id: ResolutionId::from_uuid(reader.uuid()?),
        revision: FactRevision::new(reader.u32()?).map_err(invalid)?,
    })
}
fn source(reader: &mut Reader<'_>) -> Result<TriggerSourceRef, ApplicationError> {
    Ok(match family(reader)? {
        TriggerFamily::Resolution => TriggerSourceRef::Resolution(resolution(reader)?),
        TriggerFamily::Notification => TriggerSourceRef::Notification {
            id: NotificationId::from_uuid(reader.uuid()?),
            revision: FactRevision::new(reader.u32()?).map_err(invalid)?,
            resolution: resolution(reader)?,
        },
        TriggerFamily::HearingResult => TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: HearingId::from_uuid(reader.uuid()?),
            result_id: HearingResultId::from_uuid(reader.uuid()?),
            revision: HearingResultRevision::new(reader.u32()?).map_err(invalid)?,
            agreement_id: if reader.flag()? {
                Some(HearingResultAgreementId::from_uuid(reader.uuid()?))
            } else {
                None
            },
        }),
    })
}
fn family(reader: &mut Reader<'_>) -> Result<TriggerFamily, ApplicationError> {
    match reader.byte()? {
        0 => Ok(TriggerFamily::Resolution),
        1 => Ok(TriggerFamily::Notification),
        2 => Ok(TriggerFamily::HearingResult),
        _ => Err(invalid("unknown source family")),
    }
}
fn purpose(reader: &mut Reader<'_>) -> Result<QualifiedTriggerPurpose, ApplicationError> {
    match reader.byte()? {
        0 => Ok(QualifiedTriggerPurpose::HearingEnd),
        1 => Ok(QualifiedTriggerPurpose::OrderedPeriodStart),
        _ => Err(invalid("unknown temporal purpose")),
    }
}
pub(crate) fn requirement(reader: &mut Reader<'_>) -> Result<TriggerRequirement, ApplicationError> {
    match reader.byte()? {
        0 => {
            let field = match reader.byte()? {
                0 => TriggerField::ResolutionIssuedAt,
                1 => TriggerField::NotificationPracticedAt,
                2 => TriggerField::NotificationReceivedAt,
                3 => TriggerField::NotificationStatedEffectAt,
                4 => TriggerField::HearingSessionEventTime,
                _ => return Err(invalid("unknown temporal field")),
            };
            Ok(TriggerRequirement::SourceField(field))
        }
        1 => Ok(TriggerRequirement::Qualified {
            purpose: purpose(reader)?,
            family: family(reader)?,
        }),
        _ => Err(invalid("unknown temporal requirement")),
    }
}
fn final_day(reader: &mut Reader<'_>) -> Result<FinalDayPolicy, ApplicationError> {
    Ok(if reader.flag()? {
        FinalDayPolicy::NextCountable
    } else {
        FinalDayPolicy::Preserve
    })
}
pub(crate) fn rule(reader: &mut Reader<'_>) -> Result<ArithmeticRule, ApplicationError> {
    let tag = reader.byte()?;
    let quantity =
        NonZeroU32::new(reader.u32()?).ok_or_else(|| invalid("arithmetic quantity is zero"))?;
    match tag {
        0 => Ok(ArithmeticRule::Days {
            quantity,
            inclusion: if reader.flag()? {
                DayInclusion::AfterAnchor
            } else {
                DayInclusion::OnAnchor
            },
            basis: if reader.flag()? {
                DayBasis::CalendarCountable
            } else {
                DayBasis::Natural
            },
            final_day: final_day(reader)?,
        }),
        1 => Ok(ArithmeticRule::CivilMonths {
            quantity,
            final_day: final_day(reader)?,
        }),
        2 => Ok(ArithmeticRule::ElapsedHours { quantity }),
        _ => Err(invalid("unknown arithmetic rule")),
    }
}
pub(crate) fn declared_time(
    reader: &mut Reader<'_>,
) -> Result<DeclaredProceduralTime, ApplicationError> {
    let precision = reader.byte()?;
    if precision == 0 {
        return Ok(DeclaredProceduralTime::unknown());
    }
    if !(1..=3).contains(&precision) {
        return Err(invalid("unknown temporal precision"));
    }
    let year = i32::from(reader.u16()?);
    let month = Month::try_from(reader.byte()?).map_err(invalid)?;
    let day = reader.byte()?;
    let date = CivilDate::from_date(Date::from_calendar_date(year, month, day).map_err(invalid)?)
        .map_err(invalid)?;
    let (hour, minute) = if precision >= 2 {
        (reader.byte()?, reader.byte()?)
    } else {
        (0, 0)
    };
    let second = if precision == 3 { reader.byte()? } else { 0 };
    let offset = if reader.flag()? {
        Some(UtcOffset::from_whole_seconds(reader.i32()?).map_err(invalid)?)
    } else {
        None
    };
    match precision {
        1 => DeclaredProceduralTime::date(date, offset),
        2 => DeclaredProceduralTime::minute(date, hour, minute, offset),
        3 => DeclaredProceduralTime::second(date, hour, minute, second, offset),
        _ => return Err(invalid("unknown temporal precision")),
    }
    .map_err(invalid)
}
