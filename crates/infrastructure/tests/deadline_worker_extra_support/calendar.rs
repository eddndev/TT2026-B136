use crate::{
    deadline_backend_support as dl, deadline_input_support as inputs,
    deadline_profile_database_support as profiles, judicial_calendar_database_support as calendars,
};
use application::{deadline_profiles::*, judicial_calendars::*};
use domain::{deadline_arithmetic::*, deadline_profiles::DeadlineRuleTemplate, identity::Role};
use std::num::NonZeroU32;
use time::{Time, UtcOffset};

pub fn profile(db: &dl::Fixture, calendar: &JudicialCalendarDetail) -> DeadlineProfileDetail {
    let mut input = profiles::input(Some(db.case));
    input.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
        quantity: NonZeroU32::MIN,
        inclusion: DayInclusion::AfterAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    });
    input.completion = DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            Time::from_hms(17, 30, 0).unwrap(),
            UtcOffset::from_hms(-6, 0, 0).unwrap(),
            "2026-01-01".parse().unwrap(),
            "2026-12-31".parse().unwrap(),
            dl::label("Synthetic filing channel"),
            uuid::Uuid::nil(),
        )
        .unwrap(),
    );
    input.examples[0].calendar = Some(calendar.values.clone());
    input.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
            date: "2026-01-07".parse().unwrap(),
        });
    profiles::persist(
        &profiles::service(db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: DeadlineProfileId::new(),
            change: DeadlineProfileChange::Publish {
                definition: DeadlineProfileDefinition::new(input).unwrap(),
            },
        },
    )
}

pub fn exclude_january_seventh(
    db: &dl::Fixture,
    base: &JudicialCalendarDetail,
) -> JudicialCalendarDetail {
    let original = inputs::calendar_values();
    let date = "2026-01-07".parse().unwrap();
    let values = JudicialCalendarValues::new(
        original.scope().clone(),
        original.coverage(),
        original.sources().to_vec(),
        original.weekly_pattern().to_vec(),
        vec![JudicialCalendarException::new(
            uuid::Uuid::from_u128(77),
            date,
            date,
            JudicialCalendarRule::new(
                JudicialCalendarClassification::Excluded,
                vec![uuid::Uuid::nil()],
                "Synthetic exceptional closure",
            )
            .unwrap(),
        )
        .unwrap()],
    )
    .unwrap();
    calendars::persist(
        &calendars::service(db, db.owner, Role::Owner),
        JudicialCalendarCommand {
            operation_id: JudicialCalendarOperationId::new(),
            calendar_id: base.id,
            change: JudicialCalendarChange::Replace {
                expected_revision: base.revision,
                values,
                reason: JudicialCalendarReason::new("Declare an exceptional closure").unwrap(),
            },
        },
    )
}
