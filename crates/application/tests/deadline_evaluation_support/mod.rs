#![allow(dead_code)]
#[path = "../deadline_input_support/mod.rs"]
pub mod inputs;
use application::{deadline_evaluations::*, deadline_inputs::*, deadline_profiles::*};
use domain::{
    deadline_arithmetic::*, deadline_profiles::*, deadline_triggers::*, judicial_calendars::*,
    procedural_facts::*, procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use time::{Time, UtcOffset};
use uuid::Uuid;
pub fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
pub fn text(s: &str) -> FactText {
    FactText::new(s).unwrap()
}
pub fn label(s: &str) -> FactLabel {
    FactLabel::new(s).unwrap()
}
pub fn date(s: &str) -> CivilDate {
    s.parse().unwrap()
}
pub fn n(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}
pub fn cutoff(from: &str, through: &str) -> DeadlineCompletionPolicy {
    DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            Time::from_hms(17, 30, 0).unwrap(),
            UtcOffset::from_hms(-6, 0, 0).unwrap(),
            date(from),
            date(through),
            label("Synthetic channel"),
            id(0),
        )
        .unwrap(),
    )
}
pub fn definition() -> DeadlineProfileDefinitionInput {
    DeadlineProfileDefinitionInput {
        title: label("Synthetic profile"),
        description: text("Arithmetic fixture only"),
        scope: DeadlineProfileScope::Case(inputs::case_id()),
        references: vec![JudicialCalendarSource::new(JudicialCalendarSourceInput {
            id: id(0),
            title: "Synthetic source",
            issuer: "Test fixture",
            official_url: "https://example.org/rule",
            published_on: None,
            consulted_on: date("2026-01-01"),
            locator: "Synthetic example",
        })
        .unwrap()],
        trigger: TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        template: DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
            quantity: n(2),
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        }),
        completion: cutoff("2026-01-01", "2026-12-31"),
        conditions: vec![DeadlineProfileCondition {
            id: id(1),
            statement: text("Declared condition"),
            reference_ids: vec![id(0)],
        }],
        examples: vec![DeadlineProfileExample {
            id: id(2),
            anchor: inputs::date("2026-01-06"),
            ordered_quantity: None,
            calendar: None,
            expected: DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
                date: date("2026-01-07"),
            }),
            reference_ids: vec![id(0)],
            locator: label("Two natural days including anchor"),
        }],
    }
}
pub fn fixture() -> (
    DeadlineProfileDefinitionInput,
    DeadlineEvaluationInput,
    DeadlineInputMaterial,
) {
    let source = DeadlineSourceDetail::Fact(Box::new(inputs::resolution(1, false, "2026-01-06")));
    let selection = inputs::request(&source).trigger;
    (
        definition(),
        DeadlineEvaluationInput {
            selection,
            calendar: None,
            ordered_quantity: None,
            qualification: DeadlineApplicability {
                statement: text("Declared applicability of this exact source"),
                locator: label("Operator declaration"),
                scope_applies: FactDeclaration::Known(true),
                unresolved_incident: FactDeclaration::Known(false),
                conditions: vec![DeadlineConditionAnswer {
                    id: id(1),
                    applies: FactDeclaration::Known(true),
                    locator: label("Condition evidence"),
                }],
            },
        },
        inputs::material(source),
    )
}
pub fn evaluate(
    p: DeadlineProfileDefinitionInput,
    i: &DeadlineEvaluationInput,
    m: &DeadlineInputMaterial,
) -> ProfiledDeadlineEvaluation {
    evaluate_profiled_deadline(
        inputs::hasher().as_ref(),
        &DeadlineProfileDefinition::new(p).unwrap(),
        i,
        m,
    )
    .unwrap()
}
pub fn ordered(p: &mut DeadlineProfileDefinitionInput) {
    p.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::Days {
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        },
        maximum: Some(n(6)),
    };
    p.examples[0].ordered_quantity = Some(n(2));
}
pub fn hourly(p: &mut DeadlineProfileDefinitionInput, i: &mut DeadlineEvaluationInput) {
    let at = DeclaredProceduralTime::second(
        date("2026-01-06"),
        14,
        30,
        7,
        Some(UtcOffset::from_hms(3, 0, 0).unwrap()),
    )
    .unwrap();
    p.trigger = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        family: TriggerFamily::Resolution,
    };
    p.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours { quantity: n(72) });
    p.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    p.examples[0].anchor = at;
    p.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: at.instant_value().unwrap() + time::Duration::hours(72),
        });
    i.selection.qualification = Some(QualifiedTriggerTime {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        at,
        statement: text("Declared start"),
        locator: label("Source locator"),
    });
}
