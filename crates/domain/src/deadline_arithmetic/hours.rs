use super::{ArithmeticBlock, ArithmeticOutcome, ArithmeticTraceStep};
use crate::procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime};
use std::num::NonZeroU32;
use time::{Duration, UtcOffset};

pub(super) fn elapsed_hours(
    anchor: DeclaredProceduralTime,
    quantity: NonZeroU32,
) -> (ArithmeticOutcome, Vec<ArithmeticTraceStep>) {
    match anchor.precision() {
        DeclaredProceduralPrecision::Unknown => {
            return (
                ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor),
                vec![],
            );
        }
        observed @ (DeclaredProceduralPrecision::Date | DeclaredProceduralPrecision::Minute) => {
            return (
                ArithmeticOutcome::Blocked(ArithmeticBlock::InsufficientPrecision { observed }),
                vec![],
            );
        }
        DeclaredProceduralPrecision::Second => {}
    }
    let Some(start) = anchor.instant_value() else {
        return (
            ArithmeticOutcome::Blocked(ArithmeticBlock::MissingOffset),
            vec![],
        );
    };
    // The declared second already guarantees a representable UTC start.
    // Results stay in UTC; this does not determine a future local wall time.
    let start = start.to_offset(UtcOffset::UTC);
    let candidate = start
        .checked_add(Duration::hours(i64::from(quantity.get())))
        .filter(|instant| (1..=9999).contains(&instant.year()));
    let outcome = match candidate {
        Some(instant) => ArithmeticOutcome::InstantCandidate { instant },
        None => ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted),
    };
    (
        outcome,
        vec![ArithmeticTraceStep::ElapsedHours {
            start,
            quantity,
            candidate,
        }],
    )
}
