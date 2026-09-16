use super::{
    text::required, CivilDate, JudicialCalendarClassification, MAX_JUDICIAL_CALENDAR_SOURCES,
};
use crate::DomainError;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarRule {
    classification: JudicialCalendarClassification,
    source_ids: Vec<Uuid>,
    explanation: String,
}
impl JudicialCalendarRule {
    pub fn new(
        classification: JudicialCalendarClassification,
        mut source_ids: Vec<Uuid>,
        explanation: &str,
    ) -> Result<Self, DomainError> {
        if source_ids.len() > MAX_JUDICIAL_CALENDAR_SOURCES
            || (source_ids.is_empty()
                && classification != JudicialCalendarClassification::Unresolved)
        {
            return Err(DomainError::InvalidJudicialCalendarValue("rule.source_ids"));
        }
        source_ids.sort_unstable();
        if source_ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(DomainError::InvalidJudicialCalendarValue("rule.source_ids"));
        }
        Ok(Self {
            classification,
            source_ids,
            explanation: required(explanation, 256, true, "rule.explanation")?,
        })
    }
    pub const fn classification(&self) -> JudicialCalendarClassification {
        self.classification
    }
    pub fn source_ids(&self) -> &[Uuid] {
        &self.source_ids
    }
    pub fn explanation(&self) -> &str {
        &self.explanation
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarWeekdayRule {
    weekday: u8,
    rule: JudicialCalendarRule,
}
impl JudicialCalendarWeekdayRule {
    pub fn new(weekday: u8, rule: JudicialCalendarRule) -> Result<Self, DomainError> {
        if !(1..=7).contains(&weekday) {
            return Err(DomainError::InvalidJudicialCalendarValue(
                "weekly_pattern.weekday",
            ));
        }
        Ok(Self { weekday, rule })
    }
    pub const fn weekday(&self) -> u8 {
        self.weekday
    }
    pub const fn rule(&self) -> &JudicialCalendarRule {
        &self.rule
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarException {
    id: Uuid,
    from: CivilDate,
    through: CivilDate,
    rule: JudicialCalendarRule,
}
impl JudicialCalendarException {
    pub fn new(
        id: Uuid,
        from: CivilDate,
        through: CivilDate,
        rule: JudicialCalendarRule,
    ) -> Result<Self, DomainError> {
        if through < from {
            return Err(DomainError::InvalidJudicialCalendarValue("exception.range"));
        }
        Ok(Self {
            id,
            from,
            through,
            rule,
        })
    }
    pub const fn id(&self) -> Uuid {
        self.id
    }
    pub const fn from(&self) -> CivilDate {
        self.from
    }
    pub const fn through(&self) -> CivilDate {
        self.through
    }
    pub const fn rule(&self) -> &JudicialCalendarRule {
        &self.rule
    }
}
