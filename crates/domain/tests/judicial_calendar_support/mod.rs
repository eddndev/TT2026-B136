#![allow(dead_code)]
use domain::judicial_calendars::*;
use uuid::Uuid;

pub fn date(value: &str) -> CivilDate {
    value.parse().unwrap()
}
pub fn scope() -> JudicialCalendarScope {
    JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title: "Calendar",
        jurisdiction: JudicialCalendarJurisdiction::Federal,
        entity_codes: &["09", "15"],
        authority: "Authority",
        organ: "Organ",
        territory: "Territory",
        use_description: "Declared ordinary days",
    })
    .unwrap()
}
pub fn source(id: u128) -> JudicialCalendarSource {
    JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: Uuid::from_u128(id),
        title: "Reference",
        issuer: "Issuer",
        official_url: "https://example.org/source",
        published_on: Some(date("2026-01-01")),
        consulted_on: date("2026-01-02"),
        locator: "Article 1",
    })
    .unwrap()
}
pub fn rule(classification: JudicialCalendarClassification) -> JudicialCalendarRule {
    JudicialCalendarRule::new(classification, vec![Uuid::nil()], "Declared rule").unwrap()
}
pub fn weekly() -> Vec<JudicialCalendarWeekdayRule> {
    (1..=7)
        .map(|day| {
            JudicialCalendarWeekdayRule::new(
                day,
                rule(if day <= 5 {
                    JudicialCalendarClassification::Countable
                } else {
                    JudicialCalendarClassification::Excluded
                }),
            )
            .unwrap()
        })
        .collect()
}
pub fn values(exceptions: Vec<JudicialCalendarException>) -> JudicialCalendarValues {
    JudicialCalendarValues::new(
        scope(),
        JudicialCalendarCoverage::new(date("2026-01-01"), date("2026-12-31")).unwrap(),
        vec![source(0)],
        weekly(),
        exceptions,
    )
    .unwrap()
}
