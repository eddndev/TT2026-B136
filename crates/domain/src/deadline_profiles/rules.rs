use crate::deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy};
use std::num::NonZeroU32;

/// Preserves the unit and policies while requiring a separately ordered quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderedDeadlineUnit {
    Days {
        inclusion: DayInclusion,
        basis: DayBasis,
        final_day: FinalDayPolicy,
    },
    CivilMonths {
        final_day: FinalDayPolicy,
    },
    ElapsedHours,
}

/// A maximum constrains an ordered quantity; it never supplies a missing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineRuleTemplate {
    Fixed(ArithmeticRule),
    Ordered {
        unit: OrderedDeadlineUnit,
        maximum: Option<NonZeroU32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineRuleBlock {
    MissingOrderedQuantity,
    OrderedQuantityExceedsMaximum {
        maximum: NonZeroU32,
        supplied: NonZeroU32,
    },
    UnexpectedOrderedQuantity,
}

impl DeadlineRuleTemplate {
    /// Binds only an explicitly supplied quantity and preserves every policy.
    pub const fn instantiate(
        self,
        ordered_quantity: Option<NonZeroU32>,
    ) -> Result<ArithmeticRule, DeadlineRuleBlock> {
        match (self, ordered_quantity) {
            (Self::Fixed(rule), None) => Ok(rule),
            (Self::Fixed(_), Some(_)) => Err(DeadlineRuleBlock::UnexpectedOrderedQuantity),
            (Self::Ordered { .. }, None) => Err(DeadlineRuleBlock::MissingOrderedQuantity),
            (Self::Ordered { unit, maximum }, Some(supplied)) => match maximum {
                Some(maximum) if supplied.get() > maximum.get() => {
                    Err(DeadlineRuleBlock::OrderedQuantityExceedsMaximum { maximum, supplied })
                }
                _ => Ok(unit.with_quantity(supplied)),
            },
        }
    }
}

impl OrderedDeadlineUnit {
    const fn with_quantity(self, quantity: NonZeroU32) -> ArithmeticRule {
        match self {
            Self::Days {
                inclusion,
                basis,
                final_day,
            } => ArithmeticRule::Days {
                quantity,
                inclusion,
                basis,
                final_day,
            },
            Self::CivilMonths { final_day } => ArithmeticRule::CivilMonths {
                quantity,
                final_day,
            },
            Self::ElapsedHours => ArithmeticRule::ElapsedHours { quantity },
        }
    }
}
