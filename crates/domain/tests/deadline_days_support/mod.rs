#![allow(dead_code)]
use domain::judicial_calendars::*;
use std::num::NonZeroU32;
use uuid::Uuid;

pub fn date(value: &str) -> CivilDate {
    value.parse().unwrap()
}
pub fn quantity(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}
pub fn rule(value: JudicialCalendarClassification, id: u128) -> JudicialCalendarRule {
    JudicialCalendarRule::new(value, vec![Uuid::from_u128(id)], "Synthetic declared rule").unwrap()
}
pub fn exception(
    from: &str,
    through: &str,
    value: JudicialCalendarClassification,
) -> JudicialCalendarException {
    JudicialCalendarException::new(
        Uuid::from_u128(99),
        date(from),
        date(through),
        JudicialCalendarRule::new(
            value,
            vec![Uuid::from_u128(2)],
            "Synthetic exception\nExact source",
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn calendar(
    from: &str,
    through: &str,
    weekdays: [JudicialCalendarClassification; 7],
    exceptions: Vec<JudicialCalendarException>,
) -> JudicialCalendarValues {
    let scope = JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title: "Synthetic counting fixture",
        jurisdiction: JudicialCalendarJurisdiction::Local,
        entity_codes: &["09"],
        authority: "Declared test authority",
        organ: "Declared test organ",
        territory: "Declared test territory",
        use_description: "Synthetic civil arithmetic, not a legal rule",
    })
    .unwrap();
    let sources = [1, 2]
        .into_iter()
        .map(|id| {
            JudicialCalendarSource::new(JudicialCalendarSourceInput {
                id: Uuid::from_u128(id),
                title: "Synthetic source",
                issuer: "Fixture",
                official_url: "https://example.org/fixture",
                published_on: None,
                consulted_on: date("2026-01-01"),
                locator: "Synthetic fixture only",
            })
            .unwrap()
        })
        .collect();
    let weekly = weekdays
        .into_iter()
        .enumerate()
        .map(|(i, value)| JudicialCalendarWeekdayRule::new((i + 1) as u8, rule(value, 1)).unwrap())
        .collect();
    JudicialCalendarValues::new(
        scope,
        JudicialCalendarCoverage::new(date(from), date(through)).unwrap(),
        sources,
        weekly,
        exceptions,
    )
    .unwrap()
}
pub fn weekdays() -> [JudicialCalendarClassification; 7] {
    use JudicialCalendarClassification::{Countable as C, Excluded as E};
    [C, C, C, C, C, E, E]
}
