use super::{DeadlineDayCountRecord, ProfiledDeadlineEvaluation};
use crate::deadline_evaluations::DeadlineEvaluationBlock;
use domain::{
    deadline_arithmetic::{ArithmeticOutcome, ArithmeticRule, ArithmeticTraceStep},
    deadline_triggers::{TriggerOutcome, TriggerRequirement},
    judicial_calendars::CivilDate,
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use time::OffsetDateTime;

/// Immutable evaluator output, with exact inputs and source identities stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineEvaluationRecord {
    pub(super) requirement: TriggerRequirement,
    pub(super) trigger_outcome: TriggerOutcome,
    pub(super) rule: Option<ArithmeticRule>,
    pub(super) arithmetic: Option<DeadlineArithmeticRecord>,
    pub(super) due_at: Option<OffsetDateTime>,
    pub(super) blocks: Vec<DeadlineEvaluationBlock>,
}
impl DeadlineEvaluationRecord {
    pub fn capture(value: &ProfiledDeadlineEvaluation) -> Self {
        Self {
            requirement: value.trigger().requirement(),
            trigger_outcome: *value.trigger().outcome(),
            rule: value.rule(),
            arithmetic: value.arithmetic().map(|value| DeadlineArithmeticRecord {
                rule: value.rule(),
                anchor: value.anchor(),
                outcome: *value.outcome(),
                trace: value
                    .trace()
                    .iter()
                    .map(DeadlineTraceRecord::capture)
                    .collect(),
            }),
            due_at: value.due_at(),
            blocks: value.blocks().to_vec(),
        }
    }
    pub const fn requirement(&self) -> TriggerRequirement {
        self.requirement
    }
    pub const fn trigger_outcome(&self) -> &TriggerOutcome {
        &self.trigger_outcome
    }
    pub const fn rule(&self) -> Option<ArithmeticRule> {
        self.rule
    }
    pub const fn arithmetic(&self) -> Option<&DeadlineArithmeticRecord> {
        self.arithmetic.as_ref()
    }
    pub const fn due_at(&self) -> Option<OffsetDateTime> {
        self.due_at
    }
    pub fn blocks(&self) -> &[DeadlineEvaluationBlock] {
        &self.blocks
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineArithmeticRecord {
    pub(super) rule: ArithmeticRule,
    pub(super) anchor: DeclaredProceduralTime,
    pub(super) outcome: ArithmeticOutcome,
    pub(super) trace: Vec<DeadlineTraceRecord>,
}
impl DeadlineArithmeticRecord {
    pub const fn rule(&self) -> ArithmeticRule {
        self.rule
    }
    pub const fn anchor(&self) -> DeclaredProceduralTime {
        self.anchor
    }
    pub const fn outcome(&self) -> &ArithmeticOutcome {
        &self.outcome
    }
    pub fn trace(&self) -> &[DeadlineTraceRecord] {
        &self.trace
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineTraceRecord {
    NaturalDays {
        first_included: CivilDate,
        quantity: NonZeroU32,
        candidate: Option<CivilDate>,
    },
    CivilMonths {
        anchor: CivilDate,
        quantity: NonZeroU32,
        target_year: u32,
        target_month: u8,
        requested_day: u8,
        candidate: Option<CivilDate>,
    },
    ElapsedHours {
        start: OffsetDateTime,
        quantity: NonZeroU32,
        candidate: Option<OffsetDateTime>,
    },
    CountedDays(DeadlineDayCountRecord),
    FinalDay(DeadlineDayCountRecord),
}
impl DeadlineTraceRecord {
    fn capture(value: &ArithmeticTraceStep) -> Self {
        match value {
            ArithmeticTraceStep::NaturalDays {
                first_included,
                quantity,
                candidate,
            } => Self::NaturalDays {
                first_included: *first_included,
                quantity: *quantity,
                candidate: *candidate,
            },
            ArithmeticTraceStep::CivilMonths {
                anchor,
                quantity,
                target_year,
                target_month,
                requested_day,
                candidate,
            } => Self::CivilMonths {
                anchor: *anchor,
                quantity: *quantity,
                target_year: *target_year,
                target_month: *target_month,
                requested_day: *requested_day,
                candidate: *candidate,
            },
            ArithmeticTraceStep::ElapsedHours {
                start,
                quantity,
                candidate,
            } => Self::ElapsedHours {
                start: *start,
                quantity: *quantity,
                candidate: *candidate,
            },
            ArithmeticTraceStep::CountedDays(value) => {
                Self::CountedDays(DeadlineDayCountRecord::capture(value))
            }
            ArithmeticTraceStep::FinalDay(value) => {
                Self::FinalDay(DeadlineDayCountRecord::capture(value))
            }
        }
    }
}
