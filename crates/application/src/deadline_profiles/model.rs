use super::DeadlineCivilCutoff;
use domain::{
    cases::CaseId,
    deadline_arithmetic::ArithmeticOutcome,
    deadline_profiles::{DeadlineRuleBlock, DeadlineRuleTemplate},
    deadline_triggers::TriggerRequirement,
    judicial_calendars::{JudicialCalendarScope, JudicialCalendarSource, JudicialCalendarValues},
    procedural_facts::{FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
    typed_participants::Uuid,
};
use std::num::NonZeroU32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineProfileScope {
    Global(JudicialCalendarScope),
    Case(CaseId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineCompletionPolicy {
    ArithmeticInstant,
    CivilCandidateOnly,
    CivilCutoff(DeadlineCivilCutoff),
}

/// An explicit condition whose applicability must be declared for the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileCondition {
    pub id: Uuid,
    pub statement: FactText,
    pub reference_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineExampleExpected {
    Arithmetic(ArithmeticOutcome),
    RuleBlocked(DeadlineRuleBlock),
}

/// A stated mathematical expectation, not evidence that its rule applies to a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileExample {
    pub id: Uuid,
    pub anchor: DeclaredProceduralTime,
    pub ordered_quantity: Option<NonZeroU32>,
    pub calendar: Option<JudicialCalendarValues>,
    pub expected: DeadlineExampleExpected,
    pub reference_ids: Vec<Uuid>,
    pub locator: FactLabel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileDefinitionInput {
    pub title: FactLabel,
    pub description: FactText,
    pub scope: DeadlineProfileScope,
    pub references: Vec<JudicialCalendarSource>,
    pub trigger: TriggerRequirement,
    pub template: DeadlineRuleTemplate,
    pub completion: DeadlineCompletionPolicy,
    pub conditions: Vec<DeadlineProfileCondition>,
    pub examples: Vec<DeadlineProfileExample>,
}
