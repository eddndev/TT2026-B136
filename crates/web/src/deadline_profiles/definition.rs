use super::{
    expected::Expected,
    object,
    rule::{Completion, Template, Trigger},
};
use crate::{
    error::ApiError,
    judicial_calendars::values::{Scope, Source, Values},
    procedural_facts::values::time::DeclaredTime,
};
use application::{deadline_profiles::*, ApplicationError};
use domain::{
    cases::CaseId,
    procedural_facts::{FactLabel, FactText},
};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    title: String,
    description: String,
    #[serde(deserialize_with = "object::deserialize")]
    scope: ProfileScope,
    #[serde(deserialize_with = "object::array")]
    references: Vec<Source>,
    #[serde(deserialize_with = "object::deserialize")]
    trigger: Trigger,
    #[serde(deserialize_with = "object::deserialize")]
    template: Template,
    #[serde(deserialize_with = "object::deserialize")]
    completion: Completion,
    #[serde(deserialize_with = "object::array")]
    conditions: Vec<Condition>,
    #[serde(deserialize_with = "object::array")]
    examples: Vec<Example>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ProfileScope {
    Global {
        #[serde(deserialize_with = "object::deserialize")]
        value: Scope,
    },
    Case {
        case_id: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Condition {
    id: String,
    statement: String,
    reference_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Example {
    id: String,
    #[serde(deserialize_with = "object::deserialize")]
    anchor: DeclaredTime,
    ordered_quantity: Option<u32>,
    #[serde(default, deserialize_with = "optional_object")]
    calendar: Option<Values>,
    #[serde(deserialize_with = "object::deserialize")]
    expected: Expected,
    reference_ids: Vec<String>,
    locator: String,
}
fn optional_object<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<object::Object<T>>::deserialize(d).map(|v| v.map(|v| v.0))
}
impl Definition {
    pub(super) fn validate(self) -> Result<DeadlineProfileDefinition, ApiError> {
        let scope = match self.scope {
            ProfileScope::Global { value } => DeadlineProfileScope::Global(value.validate()?),
            ProfileScope::Case { case_id } => DeadlineProfileScope::Case(CaseId::from_uuid(
                super::request::uuid(&case_id, "invalid_case_id")?,
            )),
        };
        let mut conditions = Vec::with_capacity(self.conditions.len().min(16));
        let mut examples = Vec::with_capacity(self.examples.len().min(16));
        if !(1..=16).contains(&self.conditions.len())
            || !(1..=16).contains(&self.examples.len())
            || !(1..=16).contains(&self.references.len())
        {
            return Err(invalid());
        }
        for v in self.conditions {
            conditions.push(DeadlineProfileCondition {
                id: reference(&v.id)?,
                statement: FactText::new(&v.statement).map_err(ApplicationError::from)?,
                reference_ids: references(v.reference_ids)?,
            });
        }
        for v in self.examples {
            examples.push(DeadlineProfileExample {
                id: reference(&v.id)?,
                anchor: v.anchor.validate()?,
                ordered_quantity: v.ordered_quantity.map(quantity).transpose()?,
                calendar: v.calendar.map(Values::validate).transpose()?,
                expected: v.expected.validate()?,
                reference_ids: references(v.reference_ids)?,
                locator: FactLabel::new(&v.locator).map_err(ApplicationError::from)?,
            });
        }
        DeadlineProfileDefinition::new(DeadlineProfileDefinitionInput {
            title: FactLabel::new(&self.title).map_err(ApplicationError::from)?,
            description: FactText::new(&self.description).map_err(ApplicationError::from)?,
            scope,
            references: self
                .references
                .into_iter()
                .map(Source::validate)
                .collect::<Result<Vec<_>, _>>()?,
            trigger: self.trigger.validate(),
            template: self.template.validate()?,
            completion: self.completion.validate()?,
            conditions,
            examples,
        })
        .map_err(|e| ApplicationError::from(e).into())
    }
}
pub(super) fn invalid() -> ApiError {
    ApplicationError::from(DeadlineProfileError::Invalid("definition")).into()
}
pub(super) fn quantity(v: u32) -> Result<std::num::NonZeroU32, ApiError> {
    std::num::NonZeroU32::new(v).ok_or_else(invalid)
}
pub(super) fn reference(v: &str) -> Result<uuid::Uuid, ApiError> {
    super::request::uuid(v, "invalid_deadline_profile_id")
}
fn references(v: Vec<String>) -> Result<Vec<uuid::Uuid>, ApiError> {
    if !(1..=16).contains(&v.len()) {
        return Err(invalid());
    }
    v.iter().map(|v| reference(v)).collect()
}
