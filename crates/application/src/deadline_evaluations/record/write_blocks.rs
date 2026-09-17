use super::primitives::*;
use crate::deadline_evaluations::DeadlineEvaluationBlock as EvaluationBlock;
use domain::{
    deadline_arithmetic::{ArithmeticBlock, ArithmeticOutcome},
    deadline_profiles::DeadlineRuleBlock,
    deadline_triggers::{QualifiedTriggerPurpose, TriggerBlock, TriggerFamily, TriggerField},
};
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
pub(super) fn trigger(bytes: &mut Vec<u8>, value: TriggerBlock) {
    match value {
        TriggerBlock::UnknownSource => bytes.push(0),
        TriggerBlock::AbsentField(field) => bytes.extend([
            1,
            match field {
                TriggerField::ResolutionIssuedAt => 0,
                TriggerField::NotificationPracticedAt => 1,
                TriggerField::NotificationReceivedAt => 2,
                TriggerField::NotificationStatedEffectAt => 3,
                TriggerField::HearingSessionEventTime => 4,
            },
        ]),
        TriggerBlock::IncompatibleFamily { expected, actual } => {
            bytes.extend([2, family(expected), family(actual)])
        }
        TriggerBlock::MissingQualification { purpose: p } => bytes.extend([3, purpose(p)]),
        TriggerBlock::QualificationMismatch { expected, actual } => {
            bytes.extend([4, purpose(expected), purpose(actual)])
        }
        TriggerBlock::UnexpectedQualification => bytes.push(5),
    }
}
pub(super) fn arithmetic(bytes: &mut Vec<u8>, value: ArithmeticBlock) {
    match value {
        ArithmeticBlock::UnknownAnchor => bytes.push(0),
        ArithmeticBlock::InsufficientPrecision { observed } => {
            bytes.extend([1, write_precision(observed)])
        }
        ArithmeticBlock::MissingOffset => bytes.push(2),
        ArithmeticBlock::MissingCalendar => bytes.push(3),
        ArithmeticBlock::DateRangeExhausted => bytes.push(4),
        ArithmeticBlock::MissingHomologousDay {
            year,
            month,
            requested_day,
        } => {
            bytes.push(5);
            bytes.extend(year.to_be_bytes());
            bytes.extend([month, requested_day]);
        }
        ArithmeticBlock::UnresolvedCalendarDate { date } => {
            bytes.push(6);
            write_date(bytes, date);
        }
        ArithmeticBlock::OutsideCalendarCoverage { date } => {
            bytes.push(7);
            write_date(bytes, date);
        }
    }
}
pub(super) fn outcome(bytes: &mut Vec<u8>, value: ArithmeticOutcome) {
    match value {
        ArithmeticOutcome::CivilCandidate { date } => {
            bytes.push(0);
            write_date(bytes, date);
        }
        ArithmeticOutcome::InstantCandidate { instant } => {
            bytes.push(1);
            write_instant(bytes, instant);
        }
        ArithmeticOutcome::Blocked(value) => {
            bytes.push(2);
            arithmetic(bytes, value);
        }
    }
}
fn rule(bytes: &mut Vec<u8>, value: DeadlineRuleBlock) {
    match value {
        DeadlineRuleBlock::MissingOrderedQuantity => bytes.push(0),
        DeadlineRuleBlock::OrderedQuantityExceedsMaximum { maximum, supplied } => {
            bytes.push(1);
            bytes.extend(maximum.get().to_be_bytes());
            bytes.extend(supplied.get().to_be_bytes());
        }
        DeadlineRuleBlock::UnexpectedOrderedQuantity => bytes.push(2),
    }
}
pub(super) fn evaluation(bytes: &mut Vec<u8>, value: EvaluationBlock) {
    match value {
        EvaluationBlock::ScopeUnknown => bytes.push(0),
        EvaluationBlock::ScopeRejected => bytes.push(1),
        EvaluationBlock::IncidentUnknown => bytes.push(2),
        EvaluationBlock::UnresolvedIncident => bytes.push(3),
        EvaluationBlock::ConditionMissing(id) => {
            bytes.push(4);
            bytes.extend(id.as_bytes());
        }
        EvaluationBlock::ConditionUnknown(id) => {
            bytes.push(5);
            bytes.extend(id.as_bytes());
        }
        EvaluationBlock::ConditionRejected(id) => {
            bytes.push(6);
            bytes.extend(id.as_bytes());
        }
        EvaluationBlock::Rule(value) => {
            bytes.push(7);
            rule(bytes, value);
        }
        EvaluationBlock::Trigger(value) => {
            bytes.push(8);
            trigger(bytes, value);
        }
        EvaluationBlock::Arithmetic(value) => {
            bytes.push(9);
            arithmetic(bytes, value);
        }
        EvaluationBlock::CivilCutoffMissing => bytes.push(10),
        EvaluationBlock::CutoffOutsideCoverage { candidate } => {
            bytes.push(11);
            write_date(bytes, candidate);
        }
    }
}
