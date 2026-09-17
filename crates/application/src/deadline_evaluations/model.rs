use crate::{deadline_inputs::DeadlineCalendarRef, ApplicationError};
use domain::{
    deadline_arithmetic::{ArithmeticBlock, ArithmeticRule, DeadlineArithmetic},
    deadline_profiles::DeadlineRuleBlock,
    deadline_triggers::{TriggerBlock, TriggerExtraction, TriggerSelection},
    judicial_calendars::CivilDate,
    procedural_facts::{FactDeclaration, FactLabel, FactText},
    typed_participants::Uuid,
};
use std::num::NonZeroU32;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineConditionAnswer {
    pub id: Uuid,
    pub applies: FactDeclaration<bool>,
    pub locator: FactLabel,
}

/// Operator declarations bound to the enclosing exact source selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineApplicability {
    pub statement: FactText,
    pub locator: FactLabel,
    pub scope_applies: FactDeclaration<bool>,
    pub unresolved_incident: FactDeclaration<bool>,
    pub conditions: Vec<DeadlineConditionAnswer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineEvaluationInput {
    pub selection: TriggerSelection,
    pub calendar: Option<DeadlineCalendarRef>,
    pub ordered_quantity: Option<NonZeroU32>,
    pub qualification: DeadlineApplicability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineEvaluationBlock {
    ScopeUnknown,
    ScopeRejected,
    IncidentUnknown,
    UnresolvedIncident,
    ConditionMissing(Uuid),
    ConditionUnknown(Uuid),
    ConditionRejected(Uuid),
    Rule(DeadlineRuleBlock),
    Trigger(TriggerBlock),
    Arithmetic(ArithmeticBlock),
    CivilCutoffMissing,
    CutoffOutsideCoverage { candidate: CivilDate },
}

#[derive(Debug, thiserror::Error)]
pub enum DeadlineEvaluationError {
    #[error("invalid deadline evaluation field: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Inputs(#[from] ApplicationError),
}

/// Reproduced calculation; only an authorized store can establish persisted provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfiledDeadlineEvaluation {
    pub(super) trigger: TriggerExtraction,
    pub(super) rule: Option<ArithmeticRule>,
    pub(super) arithmetic: Option<DeadlineArithmetic>,
    pub(super) due_at: Option<OffsetDateTime>,
    pub(super) blocks: Vec<DeadlineEvaluationBlock>,
}
impl ProfiledDeadlineEvaluation {
    pub fn trigger(&self) -> &TriggerExtraction {
        &self.trigger
    }
    pub const fn rule(&self) -> Option<ArithmeticRule> {
        self.rule
    }
    pub fn arithmetic(&self) -> Option<&DeadlineArithmetic> {
        self.arithmetic.as_ref()
    }
    pub const fn due_at(&self) -> Option<OffsetDateTime> {
        self.due_at
    }
    pub fn blocks(&self) -> &[DeadlineEvaluationBlock] {
        &self.blocks
    }
}
