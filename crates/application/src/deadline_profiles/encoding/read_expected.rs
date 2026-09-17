use super::{invalid, reader::Reader, DeadlineProfileError};
use crate::deadline_profiles::DeadlineExampleExpected;
use domain::{
    deadline_arithmetic::{ArithmeticBlock, ArithmeticOutcome},
    deadline_profiles::DeadlineRuleBlock,
    procedural_time::DeclaredProceduralPrecision,
};
use time::{OffsetDateTime, UtcOffset};

pub(super) fn expected(
    reader: &mut Reader<'_>,
) -> Result<DeadlineExampleExpected, DeadlineProfileError> {
    match reader.byte()? {
        0 => outcome(reader).map(DeadlineExampleExpected::Arithmetic),
        1 => rule_block(reader).map(DeadlineExampleExpected::RuleBlocked),
        _ => Err(invalid("expected tag")),
    }
}
fn outcome(reader: &mut Reader<'_>) -> Result<ArithmeticOutcome, DeadlineProfileError> {
    match reader.byte()? {
        0 => Ok(ArithmeticOutcome::CivilCandidate {
            date: reader.date()?,
        }),
        1 => {
            let seconds = reader.i64()?;
            let nanos = reader.u32()?;
            if nanos >= 1_000_000_000 {
                return Err(invalid("expected nanosecond"));
            }
            let offset = UtcOffset::from_whole_seconds(reader.i32()?).map_err(invalid)?;
            let timestamp = i128::from(seconds) * 1_000_000_000 + i128::from(nanos);
            // Expected instants retain their original offset, independently of declared-time limits.
            let instant = OffsetDateTime::from_unix_timestamp_nanos(timestamp)
                .map_err(invalid)?
                .checked_to_offset(offset)
                .ok_or_else(|| invalid("expected instant offset"))?;
            Ok(ArithmeticOutcome::InstantCandidate { instant })
        }
        2 => block(reader).map(ArithmeticOutcome::Blocked),
        _ => Err(invalid("arithmetic outcome tag")),
    }
}
fn precision(reader: &mut Reader<'_>) -> Result<DeclaredProceduralPrecision, DeadlineProfileError> {
    match reader.byte()? {
        0 => Ok(DeclaredProceduralPrecision::Unknown),
        1 => Ok(DeclaredProceduralPrecision::Date),
        2 => Ok(DeclaredProceduralPrecision::Minute),
        3 => Ok(DeclaredProceduralPrecision::Second),
        _ => Err(invalid("observed precision")),
    }
}
fn block(reader: &mut Reader<'_>) -> Result<ArithmeticBlock, DeadlineProfileError> {
    match reader.byte()? {
        0 => Ok(ArithmeticBlock::UnknownAnchor),
        1 => Ok(ArithmeticBlock::InsufficientPrecision {
            observed: precision(reader)?,
        }),
        2 => Ok(ArithmeticBlock::MissingOffset),
        3 => Ok(ArithmeticBlock::MissingCalendar),
        4 => Ok(ArithmeticBlock::DateRangeExhausted),
        5 => Ok(ArithmeticBlock::MissingHomologousDay {
            year: reader.u32()?,
            month: reader.byte()?,
            requested_day: reader.byte()?,
        }),
        6 => Ok(ArithmeticBlock::UnresolvedCalendarDate {
            date: reader.date()?,
        }),
        7 => Ok(ArithmeticBlock::OutsideCalendarCoverage {
            date: reader.date()?,
        }),
        _ => Err(invalid("arithmetic block tag")),
    }
}
fn rule_block(reader: &mut Reader<'_>) -> Result<DeadlineRuleBlock, DeadlineProfileError> {
    match reader.byte()? {
        0 => Ok(DeadlineRuleBlock::MissingOrderedQuantity),
        1 => Ok(DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
            maximum: reader.quantity()?,
            supplied: reader.quantity()?,
        }),
        2 => Ok(DeadlineRuleBlock::UnexpectedOrderedQuantity),
        _ => Err(invalid("rule block tag")),
    }
}
