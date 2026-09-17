use crate::deadline_profiles::DeadlineExampleExpected;
use domain::{
    deadline_arithmetic::{ArithmeticBlock, ArithmeticOutcome},
    deadline_profiles::DeadlineRuleBlock,
    procedural_time::DeclaredProceduralPrecision,
};

pub(super) fn expected(bytes: &mut Vec<u8>, expected: DeadlineExampleExpected) {
    match expected {
        DeadlineExampleExpected::Arithmetic(value) => {
            bytes.push(0);
            outcome(bytes, value);
        }
        DeadlineExampleExpected::RuleBlocked(value) => {
            bytes.push(1);
            rule_block(bytes, value);
        }
    }
}
fn outcome(bytes: &mut Vec<u8>, outcome: ArithmeticOutcome) {
    match outcome {
        ArithmeticOutcome::CivilCandidate { date } => {
            bytes.push(0);
            bytes.extend_from_slice(&date.days_since_epoch().to_be_bytes());
        }
        ArithmeticOutcome::InstantCandidate { instant } => {
            bytes.push(1);
            bytes.extend_from_slice(&instant.unix_timestamp().to_be_bytes());
            bytes.extend_from_slice(&instant.nanosecond().to_be_bytes());
            bytes.extend_from_slice(&instant.offset().whole_seconds().to_be_bytes());
        }
        ArithmeticOutcome::Blocked(blocked) => {
            bytes.push(2);
            block(bytes, blocked);
        }
    }
}
fn block(bytes: &mut Vec<u8>, block: ArithmeticBlock) {
    match block {
        ArithmeticBlock::UnknownAnchor => bytes.push(0),
        ArithmeticBlock::InsufficientPrecision { observed } => {
            bytes.push(1);
            bytes.push(match observed {
                DeclaredProceduralPrecision::Unknown => 0,
                DeclaredProceduralPrecision::Date => 1,
                DeclaredProceduralPrecision::Minute => 2,
                DeclaredProceduralPrecision::Second => 3,
            });
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
            bytes.extend_from_slice(&year.to_be_bytes());
            bytes.extend_from_slice(&[month, requested_day]);
        }
        ArithmeticBlock::UnresolvedCalendarDate { date } => {
            bytes.push(6);
            bytes.extend_from_slice(&date.days_since_epoch().to_be_bytes());
        }
        ArithmeticBlock::OutsideCalendarCoverage { date } => {
            bytes.push(7);
            bytes.extend_from_slice(&date.days_since_epoch().to_be_bytes());
        }
    }
}
fn rule_block(bytes: &mut Vec<u8>, block: DeadlineRuleBlock) {
    match block {
        DeadlineRuleBlock::MissingOrderedQuantity => bytes.push(0),
        DeadlineRuleBlock::OrderedQuantityExceedsMaximum { maximum, supplied } => {
            bytes.push(1);
            bytes.extend_from_slice(&maximum.get().to_be_bytes());
            bytes.extend_from_slice(&supplied.get().to_be_bytes());
        }
        DeadlineRuleBlock::UnexpectedOrderedQuantity => bytes.push(2),
    }
}
