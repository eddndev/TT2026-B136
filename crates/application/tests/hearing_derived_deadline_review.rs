use crate::deadline_support::evaluation::{self, inputs};
use crate::hearing_derived_deadline_support::*;
use application::{
    cases::CurrentCaseAdministration,
    deadline_profiles::*,
    deadline_tracking::TrackingPolicy,
    deadlines::{DeadlineId, DeadlineOperationId},
    hearing_results::*,
};
use domain::{
    deadline_triggers::{
        QualifiedTriggerPurpose, QualifiedTriggerTime, TriggerFamily, TriggerRequirement,
        TriggerSourceRef,
    },
    hearings::HearingTime,
    identity::UserId,
    procedural_facts::FactDeclaration,
};
use time::{Duration, UtcOffset};

#[derive(Debug, Clone, Copy)]
enum DecisionChange {
    ResultOperation,
    ResultIdentity,
    DeadlineOperation,
    DeadlineIdentity,
    EventDate,
    Summary,
    AgreementChoice,
    AgreementText,
    OrderedQuantity,
    ProfileRevision,
    ScopeAnswer,
    ConditionLocator,
    SourceTracking,
    ProfileTracking,
    CalendarTracking,
    ResponsibleIdentity,
    ResponsibleEmail,
    DeadlineTitle,
}

#[test]
fn a_changed_instruction_requires_new_review_even_if_its_due_time_is_unchanged() {
    let original = fixture();
    let approved = original.prepare().unwrap();
    for change in [
        DecisionChange::ResultOperation,
        DecisionChange::ResultIdentity,
        DecisionChange::DeadlineOperation,
        DecisionChange::DeadlineIdentity,
        DecisionChange::EventDate,
        DecisionChange::Summary,
        DecisionChange::AgreementChoice,
        DecisionChange::AgreementText,
        DecisionChange::OrderedQuantity,
        DecisionChange::ProfileRevision,
        DecisionChange::ScopeAnswer,
        DecisionChange::ConditionLocator,
        DecisionChange::SourceTracking,
        DecisionChange::ProfileTracking,
        DecisionChange::CalendarTracking,
        DecisionChange::ResponsibleIdentity,
        DecisionChange::ResponsibleEmail,
        DecisionChange::DeadlineTitle,
    ] {
        let mut changed = original.clone();
        mutate(&mut changed, change);
        let draft = changed
            .prepare()
            .unwrap_or_else(|error| panic!("{change:?}: {error}"));
        assert_ne!(
            approved.review_digest(),
            draft.review_digest(),
            "{change:?}"
        );
        assert!(
            draft.require_review(approved.review_digest()).is_err(),
            "{change:?}"
        );
        assert!(
            draft.require_review(draft.review_digest()).is_ok(),
            "{change:?}"
        );
    }
}

fn mutate(fixture: &mut Fixture, change: DecisionChange) {
    match change {
        DecisionChange::ResultOperation => {
            fixture.command.result.operation_id = HearingResultOperationId::new();
            fixture.refresh_result();
        }
        DecisionChange::ResultIdentity => {
            let id = HearingResultId::new();
            fixture.command.result.result_id = id;
            fixture.refresh_result();
            fixture.edit_definition(|value| {
                let FactDeclaration::Known(TriggerSourceRef::HearingResult(source)) =
                    &mut value.input.selection.source
                else {
                    panic!("hearing source expected")
                };
                source.result_id = id;
            });
        }
        DecisionChange::DeadlineOperation => fixture.edit_deadline(|command, _| {
            command.operation_id = DeadlineOperationId::new();
        }),
        DecisionChange::DeadlineIdentity => fixture.edit_deadline(|command, _| {
            command.deadline_id = DeadlineId::new();
        }),
        DecisionChange::EventDate => fixture.edit_values(|value| {
            value.event_time = DeclaredHearingResultTime::date(
                evaluation::date("2026-01-07").date(),
                UtcOffset::UTC,
            )
            .unwrap();
        }),
        DecisionChange::Summary => fixture.edit_values(|value| {
            value.summary = HearingResultText::new("Different declared summary").unwrap();
        }),
        DecisionChange::AgreementChoice => fixture.edit_definition(|value| {
            let FactDeclaration::Known(TriggerSourceRef::HearingResult(source)) =
                &mut value.input.selection.source
            else {
                panic!("hearing source expected")
            };
            source.agreement_id = Some(agreement(502));
        }),
        DecisionChange::AgreementText => fixture.edit_values(|value| {
            value.agreements[0] = HearingResultAgreement::new(
                agreement(501),
                HearingResultText::new("Revised declared agreement").unwrap(),
            );
        }),
        DecisionChange::OrderedQuantity => fixture.edit_definition(|value| {
            value.input.ordered_quantity = Some(evaluation::n(3));
        }),
        DecisionChange::ProfileRevision => {
            advance_profile(&mut fixture.material.profile);
            fixture.material.profile_head = fixture.material.profile.clone();
            fixture.edit_definition(|value| {
                value.profile.revision = DeadlineProfileRevision::new(2).unwrap()
            });
        }
        DecisionChange::ScopeAnswer => fixture.edit_definition(|value| {
            value.input.qualification.scope_applies = FactDeclaration::Known(false);
        }),
        DecisionChange::ConditionLocator => fixture.edit_definition(|value| {
            value.input.qualification.conditions[0].locator =
                evaluation::label("Different supporting passage");
        }),
        DecisionChange::SourceTracking => {
            fixture.edit_deadline(|_, policies| policies.source = TrackingPolicy::Follow)
        }
        DecisionChange::ProfileTracking => {
            fixture.edit_deadline(|_, policies| policies.profile = TrackingPolicy::Follow)
        }
        DecisionChange::CalendarTracking => {
            fixture.edit_deadline(|_, policies| policies.calendar = TrackingPolicy::Follow)
        }
        DecisionChange::ResponsibleIdentity => {
            let id = UserId::new();
            fixture.edit_definition(|value| value.responsible = id);
            fixture.material.responsible.id = id;
        }
        DecisionChange::ResponsibleEmail => {
            fixture.material.responsible.email = "other@example.com".into()
        }
        DecisionChange::DeadlineTitle => fixture
            .edit_definition(|value| value.title = evaluation::label("Other explicit consequence")),
    }
}

pub fn advance_profile(profile: &mut DeadlineProfileDetail) {
    profile.revision = DeadlineProfileRevision::new(2).unwrap();
    profile.receipt.action = DeadlineProfileAction::Replace;
    profile.receipt.expected_revision = 1;
    profile.receipt.operation_id = DeadlineProfileOperationId::new();
    profile.reason = Some(evaluation::text("A separately published profile revision"));
    resign_profile(profile);
}

#[test]
fn qualified_time_and_its_evidence_are_part_of_the_reviewed_instruction() {
    let mut original = fixture();
    original.edit_profile(|value| {
        value.trigger = TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
            family: TriggerFamily::HearingResult,
        };
    });
    original.edit_definition(|value| {
        value.input.selection.qualification = Some(QualifiedTriggerTime {
            purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
            at: inputs::date("2026-01-06"),
            statement: evaluation::text("Explicitly declared ordered period start"),
            locator: evaluation::label("First declared agreement"),
        });
    });
    let approved = original.prepare().unwrap();
    for field in ["time", "statement", "locator"] {
        let mut changed = original.clone();
        changed.edit_definition(|value| {
            let qualification = value.input.selection.qualification.as_mut().unwrap();
            match field {
                "time" => qualification.at = inputs::date("2026-01-07"),
                "statement" => {
                    qualification.statement = evaluation::text("Different explicit qualification")
                }
                _ => qualification.locator = evaluation::label("Second declared agreement"),
            }
        });
        let draft = changed.prepare().unwrap();
        assert!(
            draft.require_review(approved.review_digest()).is_err(),
            "{field}"
        );
    }
}

#[test]
fn changed_observed_projections_cannot_reuse_a_review_of_old_material() {
    let original = fixture();
    let approved = original.prepare().unwrap();
    for field in [
        "administration",
        "anchor_time",
        "profile_capture",
        "calendar_author",
    ] {
        let mut changed = original.clone();
        match field {
            "administration" => {
                let CurrentCaseAdministration::Recorded(administration) =
                    &mut changed.material.result.observed_administration
                else {
                    panic!("recorded administration expected")
                };
                administration.changed_at += Duration::seconds(1);
            }
            "anchor_time" => {
                let anchor = &mut changed.material.result.anchor;
                anchor.scheduled_at =
                    HearingTime::new(crate::case_support::instant() + Duration::hours(1)).unwrap();
            }
            "profile_capture" => {
                changed.material.profile.recorded_at += Duration::seconds(1);
                changed.material.profile_head = changed.material.profile.clone();
            }
            _ => {
                changed
                    .material
                    .calendar
                    .as_mut()
                    .unwrap()
                    .recorded_by
                    .email = "other@example.com".into();
                changed.material.calendar_head = changed.material.calendar.clone();
            }
        }
        if let Ok(draft) = changed.prepare() {
            assert!(
                draft.require_review(approved.review_digest()).is_err(),
                "{field}"
            );
        }
    }
}

#[test]
fn fixed_profile_can_keep_a_published_revision_but_follow_requires_its_head() {
    let mut fixture = fixture();
    let first = fixture.prepare().unwrap();
    advance_profile(&mut fixture.material.profile_head);
    let fixed = fixture.prepare().unwrap();
    assert_eq!(fixed.evaluation(), first.evaluation());
    assert!(fixed.require_review(first.review_digest()).is_err());
    fixture.edit_deadline(|_, policies| policies.profile = TrackingPolicy::Follow);
    assert!(fixture.prepare().is_err());
}

#[test]
fn fixed_calendar_can_keep_an_exact_revision_but_follow_requires_its_head() {
    let mut fixture = fixture();
    let first = fixture.prepare().unwrap();
    let head = inputs::calendar(
        2,
        false,
        domain::judicial_calendars::JudicialCalendarClassification::Countable,
    );
    fixture.material.calendar_head = Some(head.clone());
    let fixed = fixture.prepare().unwrap();
    assert_eq!(fixed.evaluation(), first.evaluation());
    assert!(fixed.require_review(first.review_digest()).is_err());
    fixture.edit_deadline(|_, policies| policies.calendar = TrackingPolicy::Follow);
    assert!(fixture.prepare().is_err());
    fixture.edit_definition(|value| {
        value.input.calendar.as_mut().unwrap().revision = head.revision;
    });
    fixture.material.calendar = Some(head);
    assert!(fixture.prepare().is_ok());
}
