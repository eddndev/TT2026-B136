use super::{invalid, primitives::*, Reader};
use crate::{deadline_evaluations::DeadlineEvaluationBlock as EvaluationBlock, ApplicationError};
use domain::{
    deadline_arithmetic::{ArithmeticBlock, ArithmeticOutcome},
    deadline_profiles::DeadlineRuleBlock,
    deadline_triggers::{QualifiedTriggerPurpose, TriggerBlock, TriggerFamily, TriggerField},
};

fn field(reader: &mut Reader<'_>) -> Result<TriggerField, ApplicationError> {
    match reader.byte()? {
        0 => Ok(TriggerField::ResolutionIssuedAt),
        1 => Ok(TriggerField::NotificationPracticedAt),
        2 => Ok(TriggerField::NotificationReceivedAt),
        3 => Ok(TriggerField::NotificationStatedEffectAt),
        4 => Ok(TriggerField::HearingSessionEventTime),
        _ => Err(invalid("unknown trigger field")),
    }
}
fn family(reader: &mut Reader<'_>) -> Result<TriggerFamily, ApplicationError> {
    match reader.byte()? {
        0 => Ok(TriggerFamily::Resolution),
        1 => Ok(TriggerFamily::Notification),
        2 => Ok(TriggerFamily::HearingResult),
        _ => Err(invalid("unknown trigger family")),
    }
}
fn purpose(reader: &mut Reader<'_>) -> Result<QualifiedTriggerPurpose, ApplicationError> {
    match reader.byte()? {
        0 => Ok(QualifiedTriggerPurpose::HearingEnd),
        1 => Ok(QualifiedTriggerPurpose::OrderedPeriodStart),
        _ => Err(invalid("unknown temporal purpose")),
    }
}
pub(super) fn trigger(reader: &mut Reader<'_>) -> Result<TriggerBlock, ApplicationError> {
    Ok(match reader.byte()? {
        0 => TriggerBlock::UnknownSource,
        1 => TriggerBlock::AbsentField(field(reader)?),
        2 => TriggerBlock::IncompatibleFamily {
            expected: family(reader)?,
            actual: family(reader)?,
        },
        3 => TriggerBlock::MissingQualification {
            purpose: purpose(reader)?,
        },
        4 => TriggerBlock::QualificationMismatch {
            expected: purpose(reader)?,
            actual: purpose(reader)?,
        },
        5 => TriggerBlock::UnexpectedQualification,
        _ => return Err(invalid("unknown trigger block")),
    })
}
pub(super) fn arithmetic(reader: &mut Reader<'_>) -> Result<ArithmeticBlock, ApplicationError> {
    Ok(match reader.byte()? {
        0 => ArithmeticBlock::UnknownAnchor,
        1 => ArithmeticBlock::InsufficientPrecision {
            observed: precision(reader)?,
        },
        2 => ArithmeticBlock::MissingOffset,
        3 => ArithmeticBlock::MissingCalendar,
        4 => ArithmeticBlock::DateRangeExhausted,
        5 => {
            let year = reader.u32()?;
            let month = reader.byte()?;
            let requested_day = reader.byte()?;
            if year == 0 || !(1..=12).contains(&month) || !(1..=31).contains(&requested_day) {
                return Err(invalid("invalid homologous-day fields"));
            }
            ArithmeticBlock::MissingHomologousDay {
                year,
                month,
                requested_day,
            }
        }
        6 => ArithmeticBlock::UnresolvedCalendarDate {
            date: date(reader)?,
        },
        7 => ArithmeticBlock::OutsideCalendarCoverage {
            date: date(reader)?,
        },
        _ => return Err(invalid("unknown arithmetic block")),
    })
}
pub(super) fn outcome(reader: &mut Reader<'_>) -> Result<ArithmeticOutcome, ApplicationError> {
    match reader.byte()? {
        0 => Ok(ArithmeticOutcome::CivilCandidate {
            date: date(reader)?,
        }),
        1 => Ok(ArithmeticOutcome::InstantCandidate {
            instant: instant(reader)?,
        }),
        2 => arithmetic(reader).map(ArithmeticOutcome::Blocked),
        _ => Err(invalid("unknown arithmetic outcome")),
    }
}
fn rule(reader: &mut Reader<'_>) -> Result<DeadlineRuleBlock, ApplicationError> {
    match reader.byte()? {
        0 => Ok(DeadlineRuleBlock::MissingOrderedQuantity),
        1 => {
            let maximum = quantity(reader)?;
            let supplied = quantity(reader)?;
            if supplied <= maximum {
                return Err(invalid("quantity does not exceed maximum"));
            }
            Ok(DeadlineRuleBlock::OrderedQuantityExceedsMaximum { maximum, supplied })
        }
        2 => Ok(DeadlineRuleBlock::UnexpectedOrderedQuantity),
        _ => Err(invalid("unknown rule block")),
    }
}
pub(super) fn evaluation(reader: &mut Reader<'_>) -> Result<EvaluationBlock, ApplicationError> {
    Ok(match reader.byte()? {
        0 => EvaluationBlock::ScopeUnknown,
        1 => EvaluationBlock::ScopeRejected,
        2 => EvaluationBlock::IncidentUnknown,
        3 => EvaluationBlock::UnresolvedIncident,
        4 => EvaluationBlock::ConditionMissing(reader.uuid()?),
        5 => EvaluationBlock::ConditionUnknown(reader.uuid()?),
        6 => EvaluationBlock::ConditionRejected(reader.uuid()?),
        7 => EvaluationBlock::Rule(rule(reader)?),
        8 => EvaluationBlock::Trigger(trigger(reader)?),
        9 => EvaluationBlock::Arithmetic(arithmetic(reader)?),
        10 => EvaluationBlock::CivilCutoffMissing,
        11 => EvaluationBlock::CutoffOutsideCoverage {
            candidate: date(reader)?,
        },
        _ => return Err(invalid("unknown evaluation block")),
    })
}
