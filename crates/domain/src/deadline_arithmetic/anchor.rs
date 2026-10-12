use super::{ArithmeticBlock, ArithmeticOutcome, ArithmeticTraceStep};
use crate::procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime};

pub(super) fn at_anchor(
    anchor: DeclaredProceduralTime,
) -> (ArithmeticOutcome, Vec<ArithmeticTraceStep>) {
    let block = match anchor.precision() {
        DeclaredProceduralPrecision::Unknown => Some(ArithmeticBlock::UnknownAnchor),
        observed @ (DeclaredProceduralPrecision::Date | DeclaredProceduralPrecision::Minute) => {
            Some(ArithmeticBlock::InsufficientPrecision { observed })
        }
        DeclaredProceduralPrecision::Second => None,
    };
    if let Some(block) = block {
        return (ArithmeticOutcome::Blocked(block), vec![]);
    }
    let Some(instant) = anchor.instant_value() else {
        return (
            ArithmeticOutcome::Blocked(ArithmeticBlock::MissingOffset),
            vec![],
        );
    };
    (
        ArithmeticOutcome::InstantCandidate { instant },
        vec![ArithmeticTraceStep::AtAnchor { instant }],
    )
}
