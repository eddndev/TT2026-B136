use application::deadline_profiles::*;
use domain::{
    cases::CaseId,
    deadline_arithmetic::{
        evaluate_deadline_arithmetic, ArithmeticOutcome, ArithmeticRule, FinalDayPolicy,
    },
    deadline_profiles::DeadlineRuleTemplate,
    deadline_triggers::{TriggerField, TriggerRequirement},
    judicial_calendars::*,
    procedural_facts::{FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use uuid::Uuid;

pub fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
pub fn date(value: &str) -> CivilDate {
    value.parse().unwrap()
}
pub fn quantity(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}
pub fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn source(value: u128) -> JudicialCalendarSource {
    JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: id(value),
        title: "x",
        issuer: "x",
        official_url: "https://a.aa",
        published_on: None,
        consulted_on: date("1970-01-01"),
        locator: "x",
    })
    .unwrap()
}
pub fn scope() -> JudicialCalendarScope {
    JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title: "Declared scope",
        jurisdiction: JudicialCalendarJurisdiction::Local,
        entity_codes: &["32", "01"],
        authority: "Declared authority",
        organ: "Declared organ",
        territory: "Declared territory",
        use_description: "Synthetic scope\nNo legal inference",
    })
    .unwrap()
}
pub fn calendar(classification: JudicialCalendarClassification) -> JudicialCalendarValues {
    let rule = JudicialCalendarRule::new(classification, vec![id(0)], "Synthetic day").unwrap();
    JudicialCalendarValues::new(
        scope(),
        JudicialCalendarCoverage::new(date("2026-01-01"), date("2026-12-31")).unwrap(),
        vec![source(0)],
        (1..=7)
            .map(|weekday| JudicialCalendarWeekdayRule::new(weekday, rule.clone()).unwrap())
            .collect(),
        vec![],
    )
    .unwrap()
}
pub fn input() -> DeadlineProfileDefinitionInput {
    DeadlineProfileDefinitionInput {
        title: label("x"),
        description: text("x"),
        scope: DeadlineProfileScope::Case(CaseId::from_uuid(id(0))),
        references: vec![source(0)],
        trigger: TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        template: DeadlineRuleTemplate::Fixed(ArithmeticRule::CivilMonths {
            quantity: quantity(1),
            final_day: FinalDayPolicy::Preserve,
        }),
        completion: DeadlineCompletionPolicy::CivilCandidateOnly,
        conditions: vec![DeadlineProfileCondition {
            id: id(0),
            statement: text("x"),
            reference_ids: vec![id(0)],
        }],
        examples: vec![DeadlineProfileExample {
            id: id(0),
            anchor: DeclaredProceduralTime::date(date("1970-01-01"), None).unwrap(),
            ordered_quantity: None,
            calendar: None,
            expected: DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
                date: date("1970-02-01"),
            }),
            reference_ids: vec![id(0)],
            locator: label("x"),
        }],
    }
}
pub fn recalculate(input: &mut DeadlineProfileDefinitionInput) {
    for example in &mut input.examples {
        example.expected = match input.template.instantiate(example.ordered_quantity) {
            Ok(rule) => DeadlineExampleExpected::Arithmetic(
                *evaluate_deadline_arithmetic(rule, example.anchor, example.calendar.as_ref())
                    .outcome(),
            ),
            Err(block) => DeadlineExampleExpected::RuleBlocked(block),
        };
    }
}
pub fn roundtrip(input: DeadlineProfileDefinitionInput) -> Vec<u8> {
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    let bytes = deadline_profile_definition_bytes(&profile);
    assert_eq!(&bytes[..5], b"DPRF1");
    let decoded = decode_deadline_profile_definition(&bytes).unwrap();
    assert_eq!(decoded, profile);
    for (expected, actual) in profile.examples().iter().zip(decoded.examples()) {
        if let (
            DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate { instant: a }),
            DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate { instant: b }),
        ) = (expected.expected, actual.expected)
        {
            assert_eq!(a.unix_timestamp_nanos(), b.unix_timestamp_nanos());
            assert_eq!(a.offset(), b.offset());
        }
    }
    assert_eq!(deadline_profile_definition_bytes(&decoded), bytes);
    bytes
}
