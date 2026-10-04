use crate::{
    hearings::HearingNote,
    procedural_time::{DeclaredProceduralPrecision as Precision, DeclaredProceduralTime},
    DomainError,
};

/// Original temporal components, with a reason only for an unknown declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureTime {
    declared: DeclaredProceduralTime,
    unknown_reason: Option<HearingNote>,
}

impl MeasureTime {
    pub fn new(
        declared: DeclaredProceduralTime,
        unknown_reason: Option<HearingNote>,
    ) -> Result<Self, DomainError> {
        if (declared.precision() == Precision::Unknown) != unknown_reason.is_some() {
            return Err(DomainError::InvalidPrecautionaryMeasure(
                "unknown_time_reason",
            ));
        }
        Ok(Self {
            declared,
            unknown_reason,
        })
    }

    pub const fn declared(&self) -> DeclaredProceduralTime {
        self.declared
    }

    pub fn unknown_reason(&self) -> Option<&HearingNote> {
        self.unknown_reason.as_ref()
    }
}

/// Declared terms, without a clock-driven legal status or inferred expiration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureValidity {
    start: MeasureTime,
    statement: HearingNote,
    end: Option<MeasureTime>,
}

impl MeasureValidity {
    pub fn new(
        start: MeasureTime,
        statement: HearingNote,
        end: Option<MeasureTime>,
    ) -> Result<Self, DomainError> {
        if end
            .as_ref()
            .is_some_and(|end| precedes(end.declared, start.declared))
        {
            return Err(DomainError::InvalidPrecautionaryMeasure("validity_end"));
        }
        Ok(Self {
            start,
            statement,
            end,
        })
    }

    pub const fn start(&self) -> &MeasureTime {
        &self.start
    }

    pub const fn statement(&self) -> &HearingNote {
        &self.statement
    }

    pub fn end(&self) -> Option<&MeasureTime> {
        self.end.as_ref()
    }
}

fn precedes(end: DeclaredProceduralTime, start: DeclaredProceduralTime) -> bool {
    if end.precision() != start.precision() {
        return false;
    }
    match start.precision() {
        Precision::Date if start.offset() == end.offset() => {
            matches!((end.local_date(), start.local_date()), (Some(e), Some(s)) if e < s)
        }
        Precision::Minute => {
            matches!((minute_rank(end), minute_rank(start)), (Some(e), Some(s)) if e < s)
        }
        Precision::Second => {
            matches!((end.instant_value(), start.instant_value()), (Some(e), Some(s)) if e < s)
        }
        _ => false,
    }
}

// Order equally precise declarations in UTC minutes without supplying seconds.
fn minute_rank(value: DeclaredProceduralTime) -> Option<i64> {
    let day = i64::from(value.local_date()?.date().to_julian_day());
    let hour = i64::from(value.local_hour()?);
    let minute = i64::from(value.local_minute()?);
    let offset = i64::from(value.offset()?.whole_minutes());
    Some(day * 1440 + hour * 60 + minute - offset)
}
