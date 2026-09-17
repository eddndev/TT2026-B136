#![allow(dead_code)]

mod corpus;
mod cutoff;
mod quantity_cases;

use application::deadline_profiles::{
    DeadlineCompletionPolicy, DeadlineExampleExpected, DeadlineProfileCondition,
    DeadlineProfileDefinitionInput, DeadlineProfileExample, DeadlineProfileScope,
};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{
        ArithmeticOutcome, ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy,
    },
    deadline_profiles::DeadlineRuleTemplate,
    deadline_triggers::{TriggerField, TriggerRequirement},
    judicial_calendars::{
        CivilDate, JudicialCalendarScope, JudicialCalendarSource, JudicialCalendarSourceInput,
        JudicialCalendarValues,
    },
    procedural_facts::{FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use uuid::Uuid;

#[path = "../../../domain/tests/deadline_days_support/mod.rs"]
mod calendar_support;

pub fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
pub fn date(value: &str) -> CivilDate {
    value.parse().unwrap()
}
pub fn quantity(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}
pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
pub fn source(value: u128) -> JudicialCalendarSource {
    JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: id(value),
        title: "Synthetic reference",
        issuer: "Test fixture, not a legal authority",
        official_url: "https://example.org/synthetic",
        published_on: None,
        consulted_on: date("2026-01-01"),
        locator: "Synthetic example only",
    })
    .unwrap()
}
pub fn calendar() -> JudicialCalendarValues {
    calendar_support::calendar(
        "2026-01-01",
        "2026-12-31",
        calendar_support::weekdays(),
        vec![],
    )
}
pub fn scope() -> JudicialCalendarScope {
    calendar().scope().clone()
}
pub fn daily() -> ArithmeticRule {
    ArithmeticRule::Days {
        quantity: quantity(2),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::Natural,
        final_day: FinalDayPolicy::Preserve,
    }
}
pub fn example(value: u128) -> DeadlineProfileExample {
    DeadlineProfileExample {
        id: id(value),
        anchor: DeclaredProceduralTime::date(date("2026-01-06"), None).unwrap(),
        ordered_quantity: None,
        calendar: None,
        expected: DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-07"),
        }),
        reference_ids: vec![id(0)],
        locator: label("Synthetic case: two natural days with anchor included"),
    }
}
pub fn condition(value: u128) -> DeadlineProfileCondition {
    DeadlineProfileCondition {
        id: id(value),
        statement: text("The operator declares that this synthetic condition is satisfied"),
        reference_ids: vec![id(0)],
    }
}
pub fn input() -> DeadlineProfileDefinitionInput {
    DeadlineProfileDefinitionInput {
        title: label("Synthetic declared profile"),
        description: text("Fixture for explicit arithmetic, not a legal interpretation"),
        scope: DeadlineProfileScope::Case(CaseId::from_uuid(id(50))),
        references: vec![source(0)],
        trigger: TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        template: DeadlineRuleTemplate::Fixed(daily()),
        completion: DeadlineCompletionPolicy::CivilCandidateOnly,
        conditions: vec![condition(1)],
        examples: vec![example(2)],
    }
}
