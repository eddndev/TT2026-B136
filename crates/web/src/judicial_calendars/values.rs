use application::{judicial_calendars::*, ApplicationError};
use domain::DomainError;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Values {
    #[serde(deserialize_with = "super::object::deserialize")]
    scope: Scope,
    #[serde(deserialize_with = "super::object::deserialize")]
    coverage: Coverage,
    #[serde(deserialize_with = "super::object::array")]
    sources: Vec<Source>,
    #[serde(deserialize_with = "super::object::array")]
    weekly_pattern: Vec<Weekday>,
    #[serde(deserialize_with = "super::object::array")]
    exceptions: Vec<Exception>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scope {
    title: String,
    jurisdiction: String,
    entity_codes: Vec<String>,
    authority: String,
    organ: String,
    territory: String,
    use_description: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverage {
    from: String,
    through: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Source {
    id: String,
    title: String,
    issuer: String,
    official_url: String,
    published_on: Option<String>,
    consulted_on: String,
    locator: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Weekday {
    weekday: u8,
    classification: String,
    source_ids: Vec<String>,
    explanation: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exception {
    id: String,
    from: String,
    through: String,
    classification: String,
    source_ids: Vec<String>,
    explanation: String,
}
impl Values {
    pub fn validate(self) -> Result<JudicialCalendarValues, ApplicationError> {
        let scope = self.scope.validate()?;
        let coverage = JudicialCalendarCoverage::new(
            self.coverage.from.parse()?,
            self.coverage.through.parse()?,
        )?;
        let sources = self
            .sources
            .into_iter()
            .map(Source::validate)
            .collect::<Result<Vec<_>, _>>()?;
        let weekly = self
            .weekly_pattern
            .into_iter()
            .map(|r| {
                JudicialCalendarWeekdayRule::new(
                    r.weekday,
                    rule(&r.classification, r.source_ids, &r.explanation)?,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let exceptions = self
            .exceptions
            .into_iter()
            .map(|e| {
                JudicialCalendarException::new(
                    uuid(&e.id)?,
                    e.from.parse()?,
                    e.through.parse()?,
                    rule(&e.classification, e.source_ids, &e.explanation)?,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(JudicialCalendarValues::new(
            scope, coverage, sources, weekly, exceptions,
        )?)
    }
}
fn uuid(value: &str) -> Result<Uuid, DomainError> {
    if value.len() > 36 {
        return Err(DomainError::InvalidJudicialCalendarValue("UUID"));
    }
    Uuid::parse_str(value).map_err(|_| DomainError::InvalidJudicialCalendarValue("UUID"))
}
fn rule(
    classification: &str,
    ids: Vec<String>,
    explanation: &str,
) -> Result<JudicialCalendarRule, DomainError> {
    JudicialCalendarRule::new(
        classification.parse()?,
        ids.iter().map(|s| uuid(s)).collect::<Result<Vec<_>, _>>()?,
        explanation,
    )
}

impl Scope {
    pub(crate) fn validate(self) -> Result<JudicialCalendarScope, ApplicationError> {
        Ok(JudicialCalendarScope::new(JudicialCalendarScopeInput {
            title: &self.title,
            jurisdiction: self.jurisdiction.parse()?,
            entity_codes: &self
                .entity_codes
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            authority: &self.authority,
            organ: &self.organ,
            territory: &self.territory,
            use_description: &self.use_description,
        })?)
    }
}
impl Source {
    pub(crate) fn validate(self) -> Result<JudicialCalendarSource, ApplicationError> {
        Ok(JudicialCalendarSource::new(JudicialCalendarSourceInput {
            id: uuid(&self.id)?,
            title: &self.title,
            issuer: &self.issuer,
            official_url: &self.official_url,
            published_on: self.published_on.map(|s| s.parse()).transpose()?,
            consulted_on: self.consulted_on.parse()?,
            locator: &self.locator,
        })?)
    }
}
