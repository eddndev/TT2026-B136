use super::{
    object::Object,
    request::{checked, invalid, uuid},
    selection::{Declaration, Selection},
};
use crate::error::ApiError;
use application::{
    deadline_evaluations::*,
    deadline_inputs::DeadlineCalendarRef,
    deadline_profiles::{DeadlineProfileId, DeadlineProfileRevision},
    deadlines::*,
};
use domain::{
    identity::UserId,
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::{FactLabel, FactText},
};
use serde::Deserialize;
use std::{collections::HashSet, num::NonZeroU32};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    title: String,
    profile: Object<Profile>,
    input: Object<Input>,
    responsible_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    id: String,
    revision: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Calendar {
    id: String,
    revision: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    selection: Object<Selection>,
    calendar: Option<Object<Calendar>>,
    ordered_quantity: Option<u32>,
    qualification: Object<Applicability>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Applicability {
    statement: String,
    locator: String,
    scope_applies: Object<Declaration<bool>>,
    unresolved_incident: Object<Declaration<bool>>,
    conditions: Vec<Object<Condition>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Condition {
    id: String,
    applies: Object<Declaration<bool>>,
    locator: String,
}
impl Definition {
    pub(super) fn validate(self) -> Result<DeadlineDefinition, ApiError> {
        Ok(DeadlineDefinition {
            title: checked(FactLabel::new(&self.title))?,
            profile: DeadlineProfileRef {
                id: DeadlineProfileId::from_uuid(uuid(
                    &self.profile.0.id,
                    "invalid_deadline_profile_id",
                )?),
                revision: checked(DeadlineProfileRevision::new(self.profile.0.revision))?,
            },
            input: self.input.0.validate()?,
            responsible: UserId::from_uuid(uuid(&self.responsible_id, "invalid_user_id")?),
        })
    }
}
impl Input {
    fn validate(self) -> Result<DeadlineEvaluationInput, ApiError> {
        Ok(DeadlineEvaluationInput {
            selection: self.selection.0.validate()?,
            calendar: self
                .calendar
                .map(|v| {
                    Ok::<_, ApiError>(DeadlineCalendarRef {
                        id: JudicialCalendarId::from_uuid(uuid(
                            &v.0.id,
                            "invalid_judicial_calendar_id",
                        )?),
                        revision: checked(JudicialCalendarRevision::new(v.0.revision))?,
                    })
                })
                .transpose()?,
            ordered_quantity: self
                .ordered_quantity
                .map(|v| NonZeroU32::new(v).ok_or_else(invalid))
                .transpose()?,
            qualification: self.qualification.0.validate()?,
        })
    }
}
impl Applicability {
    fn validate(self) -> Result<DeadlineApplicability, ApiError> {
        if self.conditions.len() > 16 {
            return Err(invalid());
        }
        let mut ids = HashSet::new();
        let mut conditions = Vec::with_capacity(self.conditions.len());
        for value in self.conditions {
            let value = value.0;
            let id = uuid(&value.id, "invalid_deadline_condition_id")?;
            if !ids.insert(id) {
                return Err(invalid());
            }
            conditions.push(DeadlineConditionAnswer {
                id,
                applies: value.applies.0.validate(Ok)?,
                locator: checked(FactLabel::new(&value.locator))?,
            });
        }
        Ok(DeadlineApplicability {
            statement: checked(FactText::new(&self.statement))?,
            locator: checked(FactLabel::new(&self.locator))?,
            scope_applies: self.scope_applies.0.validate(Ok)?,
            unresolved_incident: self.unresolved_incident.0.validate(Ok)?,
            conditions,
        })
    }
}
