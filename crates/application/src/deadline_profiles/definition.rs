use super::*;
use domain::{
    deadline_profiles::DeadlineRuleTemplate,
    deadline_triggers::TriggerRequirement,
    judicial_calendars::JudicialCalendarSource,
    procedural_facts::{FactLabel, FactText},
};

/// Normalized values whose stated examples reproduce without legal inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileDefinition {
    values: DeadlineProfileDefinitionInput,
}
impl DeadlineProfileDefinition {
    pub fn new(values: DeadlineProfileDefinitionInput) -> Result<Self, DeadlineProfileError> {
        super::validation::validate(&values)?;
        Ok(Self { values })
    }
    pub fn title(&self) -> &FactLabel {
        &self.values.title
    }
    pub fn description(&self) -> &FactText {
        &self.values.description
    }
    pub fn scope(&self) -> &DeadlineProfileScope {
        &self.values.scope
    }
    pub fn references(&self) -> &[JudicialCalendarSource] {
        &self.values.references
    }
    pub const fn trigger(&self) -> TriggerRequirement {
        self.values.trigger
    }
    pub const fn template(&self) -> DeadlineRuleTemplate {
        self.values.template
    }
    pub fn completion(&self) -> &DeadlineCompletionPolicy {
        &self.values.completion
    }
    pub fn conditions(&self) -> &[DeadlineProfileCondition] {
        &self.values.conditions
    }
    pub fn examples(&self) -> &[DeadlineProfileExample] {
        &self.values.examples
    }
}
