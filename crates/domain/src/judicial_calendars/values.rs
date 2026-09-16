use super::*;
use crate::DomainError;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarValues {
    scope: JudicialCalendarScope,
    coverage: JudicialCalendarCoverage,
    sources: Vec<JudicialCalendarSource>,
    weekly_pattern: Vec<JudicialCalendarWeekdayRule>,
    exceptions: Vec<JudicialCalendarException>,
}
impl JudicialCalendarValues {
    pub fn new(
        scope: JudicialCalendarScope,
        coverage: JudicialCalendarCoverage,
        mut sources: Vec<JudicialCalendarSource>,
        mut weekly_pattern: Vec<JudicialCalendarWeekdayRule>,
        mut exceptions: Vec<JudicialCalendarException>,
    ) -> Result<Self, DomainError> {
        if sources.len() > MAX_JUDICIAL_CALENDAR_SOURCES {
            return Err(invalid("sources"));
        }
        sources.sort_unstable_by_key(|s| s.id());
        if sources.windows(2).any(|w| w[0].id() == w[1].id()) {
            return Err(invalid("sources"));
        }
        if weekly_pattern.len() != 7 {
            return Err(invalid("weekly_pattern"));
        }
        weekly_pattern.sort_unstable_by_key(|w| w.weekday());
        if weekly_pattern
            .windows(2)
            .any(|w| w[0].weekday() == w[1].weekday())
        {
            return Err(invalid("weekly_pattern"));
        }
        if exceptions.len() > MAX_JUDICIAL_CALENDAR_EXCEPTIONS {
            return Err(invalid("exceptions"));
        }
        exceptions.sort_unstable_by_key(|e| (e.from(), e.through(), e.id()));
        let mut ids = HashSet::new();
        if exceptions.iter().any(|e| {
            !ids.insert(e.id()) || !coverage.contains(e.from()) || !coverage.contains(e.through())
        }) || exceptions.windows(2).any(|w| w[0].through() >= w[1].from())
        {
            return Err(invalid("exceptions"));
        }
        for rule in weekly_pattern
            .iter()
            .map(|w| w.rule())
            .chain(exceptions.iter().map(|e| e.rule()))
        {
            if rule
                .source_ids()
                .iter()
                .any(|id| !sources.iter().any(|s| s.id() == *id))
            {
                return Err(invalid("rule.source_ids"));
            }
        }
        Ok(Self {
            scope,
            coverage,
            sources,
            weekly_pattern,
            exceptions,
        })
    }
    pub const fn scope(&self) -> &JudicialCalendarScope {
        &self.scope
    }
    pub const fn coverage(&self) -> JudicialCalendarCoverage {
        self.coverage
    }
    pub fn sources(&self) -> &[JudicialCalendarSource] {
        &self.sources
    }
    pub fn weekly_pattern(&self) -> &[JudicialCalendarWeekdayRule] {
        &self.weekly_pattern
    }
    pub fn exceptions(&self) -> &[JudicialCalendarException] {
        &self.exceptions
    }
    pub fn has_unresolved(&self) -> bool {
        self.weekly_pattern
            .iter()
            .map(|w| w.rule())
            .chain(self.exceptions.iter().map(|e| e.rule()))
            .any(|r| r.classification() == JudicialCalendarClassification::Unresolved)
    }
    pub fn classify(&self, date: CivilDate) -> JudicialCalendarDay {
        if !self.coverage.contains(date) {
            return JudicialCalendarDay {
                date,
                origin: None,
                rule: None,
            };
        }
        if let Some(exception) = self
            .exceptions
            .iter()
            .find(|e| e.from() <= date && date <= e.through())
        {
            return JudicialCalendarDay {
                date,
                origin: Some(JudicialCalendarDayOrigin::Exception(exception.id())),
                rule: Some(exception.rule().clone()),
            };
        }
        let weekday = date.weekday();
        JudicialCalendarDay {
            date,
            origin: Some(JudicialCalendarDayOrigin::WeeklyPattern(weekday)),
            rule: Some(self.weekly_pattern[usize::from(weekday - 1)].rule().clone()),
        }
    }
}
fn invalid(field: &'static str) -> DomainError {
    DomainError::InvalidJudicialCalendarValue(field)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudicialCalendarDayOrigin {
    WeeklyPattern(u8),
    Exception(Uuid),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarDay {
    date: CivilDate,
    origin: Option<JudicialCalendarDayOrigin>,
    rule: Option<JudicialCalendarRule>,
}
impl JudicialCalendarDay {
    pub const fn date(&self) -> CivilDate {
        self.date
    }
    pub const fn origin(&self) -> Option<JudicialCalendarDayOrigin> {
        self.origin
    }
    pub fn classification(&self) -> Option<JudicialCalendarClassification> {
        self.rule.as_ref().map(|r| r.classification())
    }
    pub fn explanation(&self) -> Option<&str> {
        self.rule.as_ref().map(|r| r.explanation())
    }
    pub fn source_ids(&self) -> &[Uuid] {
        self.rule
            .as_ref()
            .map(|r| r.source_ids())
            .unwrap_or_default()
    }
}
