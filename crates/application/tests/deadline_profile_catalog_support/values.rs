use application::deadline_profiles::*;
use domain::{
    cases::CaseId,
    deadline_arithmetic::{
        ArithmeticOutcome, ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy,
    },
    deadline_profiles::DeadlineRuleTemplate,
    deadline_triggers::{TriggerField, TriggerRequirement},
    judicial_calendars::{
        CivilDate, JudicialCalendarJurisdiction, JudicialCalendarScope, JudicialCalendarScopeInput,
        JudicialCalendarSource, JudicialCalendarSourceInput,
    },
    procedural_facts::{FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
use uuid::Uuid;

pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn definition(case: Option<CaseId>) -> DeadlineProfileDefinition {
    DeadlineProfileDefinition::new(input(case)).unwrap()
}
pub fn input(case: Option<CaseId>) -> DeadlineProfileDefinitionInput {
    let source_id = Uuid::nil();
    let date: CivilDate = "2026-01-06".parse().unwrap();
    DeadlineProfileDefinitionInput {
        title: FactLabel::new("Synthetic rule").unwrap(),
        description: text("Mathematical fixture without legal claims"),
        scope: case.map(DeadlineProfileScope::Case).unwrap_or_else(|| {
            DeadlineProfileScope::Global(
                JudicialCalendarScope::new(JudicialCalendarScopeInput {
                    title: "Declared scope",
                    jurisdiction: JudicialCalendarJurisdiction::Federal,
                    entity_codes: &["09"],
                    authority: "Synthetic authority",
                    organ: "Synthetic organ",
                    territory: "Declared territory",
                    use_description: "Test configuration",
                })
                .unwrap(),
            )
        }),
        references: vec![JudicialCalendarSource::new(JudicialCalendarSourceInput {
            id: source_id,
            title: "Synthetic reference",
            issuer: "Test fixture",
            official_url: "https://example.org/synthetic",
            published_on: None,
            consulted_on: date,
            locator: "Mathematical example",
        })
        .unwrap()],
        trigger: TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        template: DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
            quantity: NonZeroU32::MIN,
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        }),
        completion: DeadlineCompletionPolicy::CivilCandidateOnly,
        conditions: vec![DeadlineProfileCondition {
            id: Uuid::from_u128(1),
            statement: text("Operator must declare applicability"),
            reference_ids: vec![source_id],
        }],
        examples: vec![DeadlineProfileExample {
            id: Uuid::from_u128(2),
            anchor: DeclaredProceduralTime::date(date, None).unwrap(),
            ordered_quantity: None,
            calendar: None,
            expected: DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
                date,
            }),
            reference_ids: vec![source_id],
            locator: FactLabel::new("One included natural day").unwrap(),
        }],
    }
}

pub fn altered_global_scope() -> DeadlineProfileScope {
    DeadlineProfileScope::Global(
        JudicialCalendarScope::new(JudicialCalendarScopeInput {
            title: "Declared scope",
            jurisdiction: JudicialCalendarJurisdiction::Federal,
            entity_codes: &["09"],
            authority: "Different authority",
            organ: "Synthetic organ",
            territory: "Declared territory",
            use_description: "Test configuration",
        })
        .unwrap(),
    )
}
