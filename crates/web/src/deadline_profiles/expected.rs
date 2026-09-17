use super::{
    definition::{invalid, quantity},
    object,
};
use crate::error::ApiError;
use application::deadline_profiles::DeadlineExampleExpected;
use domain::{
    deadline_arithmetic::{ArithmeticBlock, ArithmeticOutcome},
    deadline_profiles::DeadlineRuleBlock,
    procedural_time::DeclaredProceduralPrecision,
};
use serde::Deserialize;
use serde_json::{json, Value};
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Expected {
    Arithmetic {
        #[serde(deserialize_with = "object::deserialize")]
        outcome: Outcome,
    },
    RuleBlocked {
        #[serde(deserialize_with = "object::deserialize")]
        block: RuleBlock,
    },
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    CivilCandidate {
        date: String,
    },
    InstantCandidate {
        #[serde(deserialize_with = "object::deserialize")]
        instant: Instant,
    },
    Blocked {
        #[serde(deserialize_with = "object::deserialize")]
        block: Block,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Instant {
    unix_seconds: i64,
    nanosecond: u32,
    offset_seconds: i32,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Block {
    UnknownAnchor {},
    InsufficientPrecision {
        observed: Precision,
    },
    MissingOffset {},
    MissingCalendar {},
    DateRangeExhausted {},
    MissingHomologousDay {
        year: u32,
        month: u8,
        requested_day: u8,
    },
    UnresolvedCalendarDate {
        date: String,
    },
    OutsideCalendarCoverage {
        date: String,
    },
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Precision {
    Unknown,
    Date,
    Minute,
    Second,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RuleBlock {
    MissingOrderedQuantity {},
    UnexpectedOrderedQuantity {},
    OrderedQuantityExceedsMaximum { maximum: u32, supplied: u32 },
}
impl Expected {
    pub(super) fn validate(self) -> Result<DeadlineExampleExpected, ApiError> {
        Ok(match self {
            Self::Arithmetic { outcome } => DeadlineExampleExpected::Arithmetic(match outcome {
                Outcome::CivilCandidate { date } => ArithmeticOutcome::CivilCandidate {
                    date: date.parse().map_err(|_| invalid())?,
                },
                Outcome::InstantCandidate { instant: v } => {
                    let instant = time::OffsetDateTime::from_unix_timestamp(v.unix_seconds)
                        .map_err(|_| invalid())?
                        .replace_nanosecond(v.nanosecond)
                        .map_err(|_| invalid())?
                        .checked_to_offset(
                            time::UtcOffset::from_whole_seconds(v.offset_seconds)
                                .map_err(|_| invalid())?,
                        )
                        .ok_or_else(invalid)?;
                    ArithmeticOutcome::InstantCandidate { instant }
                }
                Outcome::Blocked { block } => ArithmeticOutcome::Blocked(block.validate()?),
            }),
            Self::RuleBlocked { block } => DeadlineExampleExpected::RuleBlocked(match block {
                RuleBlock::MissingOrderedQuantity {} => DeadlineRuleBlock::MissingOrderedQuantity,
                RuleBlock::UnexpectedOrderedQuantity {} => {
                    DeadlineRuleBlock::UnexpectedOrderedQuantity
                }
                RuleBlock::OrderedQuantityExceedsMaximum { maximum, supplied } => {
                    DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
                        maximum: quantity(maximum)?,
                        supplied: quantity(supplied)?,
                    }
                }
            }),
        })
    }
}
impl Block {
    fn validate(self) -> Result<ArithmeticBlock, ApiError> {
        Ok(match self {
            Self::UnknownAnchor {} => ArithmeticBlock::UnknownAnchor,
            Self::MissingOffset {} => ArithmeticBlock::MissingOffset,
            Self::MissingCalendar {} => ArithmeticBlock::MissingCalendar,
            Self::DateRangeExhausted {} => ArithmeticBlock::DateRangeExhausted,
            Self::InsufficientPrecision { observed } => ArithmeticBlock::InsufficientPrecision {
                observed: match observed {
                    Precision::Unknown => DeclaredProceduralPrecision::Unknown,
                    Precision::Date => DeclaredProceduralPrecision::Date,
                    Precision::Minute => DeclaredProceduralPrecision::Minute,
                    Precision::Second => DeclaredProceduralPrecision::Second,
                },
            },
            Self::MissingHomologousDay {
                year,
                month,
                requested_day,
            } => ArithmeticBlock::MissingHomologousDay {
                year,
                month,
                requested_day,
            },
            Self::UnresolvedCalendarDate { date } => ArithmeticBlock::UnresolvedCalendarDate {
                date: date.parse().map_err(|_| invalid())?,
            },
            Self::OutsideCalendarCoverage { date } => ArithmeticBlock::OutsideCalendarCoverage {
                date: date.parse().map_err(|_| invalid())?,
            },
        })
    }
}
pub(super) fn project(v: DeadlineExampleExpected) -> Value {
    match v {
        DeadlineExampleExpected::Arithmetic(outcome) => {
            json!({"kind":"arithmetic","outcome":match outcome {
                ArithmeticOutcome::CivilCandidate{date}=>json!({"kind":"civil_candidate","date":date.to_string()}),
                ArithmeticOutcome::InstantCandidate{instant}=>json!({"kind":"instant_candidate","instant":{"unix_seconds":instant.unix_timestamp(),"nanosecond":instant.nanosecond(),"offset_seconds":instant.offset().whole_seconds()}}),
                ArithmeticOutcome::Blocked(b)=>json!({"kind":"blocked","block":block(b)}),
            }})
        }
        DeadlineExampleExpected::RuleBlocked(b) => json!({"kind":"rule_blocked","block":match b {
            DeadlineRuleBlock::MissingOrderedQuantity=>json!({"kind":"missing_ordered_quantity"}),
            DeadlineRuleBlock::UnexpectedOrderedQuantity=>json!({"kind":"unexpected_ordered_quantity"}),
            DeadlineRuleBlock::OrderedQuantityExceedsMaximum{maximum,supplied}=>json!({"kind":"ordered_quantity_exceeds_maximum","maximum":maximum.get(),"supplied":supplied.get()}),
        }}),
    }
}
fn block(v: ArithmeticBlock) -> Value {
    match v {
        ArithmeticBlock::UnknownAnchor => json!({"kind":"unknown_anchor"}),
        ArithmeticBlock::MissingOffset => json!({"kind":"missing_offset"}),
        ArithmeticBlock::MissingCalendar => json!({"kind":"missing_calendar"}),
        ArithmeticBlock::DateRangeExhausted => json!({"kind":"date_range_exhausted"}),
        ArithmeticBlock::InsufficientPrecision { observed } => {
            json!({"kind":"insufficient_precision","observed":match observed {
        DeclaredProceduralPrecision::Unknown=>"unknown",DeclaredProceduralPrecision::Date=>"date",DeclaredProceduralPrecision::Minute=>"minute",DeclaredProceduralPrecision::Second=>"second"}})
        }
        ArithmeticBlock::MissingHomologousDay {
            year,
            month,
            requested_day,
        } => {
            json!({"kind":"missing_homologous_day","year":year,"month":month,"requested_day":requested_day})
        }
        ArithmeticBlock::UnresolvedCalendarDate { date } => {
            json!({"kind":"unresolved_calendar_date","date":date.to_string()})
        }
        ArithmeticBlock::OutsideCalendarCoverage { date } => {
            json!({"kind":"outside_calendar_coverage","date":date.to_string()})
        }
    }
}
