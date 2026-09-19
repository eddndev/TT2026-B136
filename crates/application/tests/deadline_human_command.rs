#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;

use application::{
    deadline_inputs::DeadlineCalendarRef,
    deadline_tracking::{TrackingPolicies, TrackingPolicy},
    deadlines::*,
};
use deadline_support::{attention, evaluation::text, fixture};
use domain::{
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::FactDeclaration,
};
use uuid::Uuid;

fn qualifications() -> [DeadlineCommand; 2] {
    let (register, _) = fixture();
    let DeadlineChange::Register { definition } = &register.change else {
        unreachable!()
    };
    let correct = DeadlineCommand {
        operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(102)),
        deadline_id: register.deadline_id,
        change: DeadlineChange::Correct {
            expected_revision: DeadlineRevision::new(3).unwrap(),
            definition: definition.clone(),
            reason: text("Reviewed the exact selected sources"),
        },
    };
    [register, correct]
}

fn definition_mut(command: &mut DeadlineCommand) -> &mut DeadlineDefinition {
    match &mut command.change {
        DeadlineChange::Register { definition } | DeadlineChange::Correct { definition, .. } => {
            definition
        }
        _ => unreachable!(),
    }
}

fn policies() -> TrackingPolicies {
    TrackingPolicies {
        profile: TrackingPolicy::Fixed,
        source: TrackingPolicy::Follow,
        calendar: TrackingPolicy::Undetermined,
    }
}

fn select_calendar(command: &mut DeadlineCommand) {
    definition_mut(command).input.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::from_uuid(Uuid::from_u128(400)),
        revision: JudicialCalendarRevision::new(3).unwrap(),
    });
}

fn unknown_source(command: &mut DeadlineCommand) {
    definition_mut(command).input.selection.source =
        FactDeclaration::Unknown(text("The exact source has not been identified"));
}

fn rejects(command: DeadlineCommand, policies: Option<TrackingPolicies>) {
    let action = command.action();
    let result: Result<DeadlineHumanCommand, DeadlineError> =
        DeadlineHumanCommand::new(command, policies);
    assert!(
        matches!(result, Err(DeadlineError::Invalid(_))),
        "{action:?} must reject invalid policies {policies:?} as input"
    );
}

fn preserves(command: DeadlineCommand, policies: Option<TrackingPolicies>) {
    let value = DeadlineHumanCommand::new(command.clone(), policies).unwrap();
    let (actual_command, actual_policies) = value.into_parts();
    assert_eq!(actual_command, command);
    assert_eq!(actual_policies, policies);
}

#[test]
fn register_and_correct_require_explicit_policies() {
    for command in qualifications() {
        rejects(command, None);
    }
}

#[test]
fn register_and_correct_cannot_leave_profile_policy_undetermined() {
    for command in qualifications() {
        rejects(
            command,
            Some(TrackingPolicies {
                profile: TrackingPolicy::Undetermined,
                ..policies()
            }),
        );
    }
}

#[test]
fn a_known_source_requires_a_declared_policy() {
    for mut command in qualifications() {
        assert!(matches!(
            definition_mut(&mut command).input.selection.source,
            FactDeclaration::Known(_)
        ));
        rejects(
            command,
            Some(TrackingPolicies {
                source: TrackingPolicy::Undetermined,
                ..policies()
            }),
        );
    }
}

#[test]
fn a_selected_calendar_requires_a_declared_policy() {
    for mut command in qualifications() {
        select_calendar(&mut command);
        rejects(command, Some(policies()));
    }
}

#[test]
fn an_unknown_source_cannot_be_fixed_or_followed() {
    for mut command in qualifications() {
        unknown_source(&mut command);
        for source in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
            rejects(
                command.clone(),
                Some(TrackingPolicies {
                    source,
                    ..policies()
                }),
            );
        }
    }
}

#[test]
fn an_absent_calendar_cannot_be_fixed_or_followed() {
    for mut command in qualifications() {
        assert!(definition_mut(&mut command).input.calendar.is_none());
        for calendar in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
            rejects(
                command.clone(),
                Some(TrackingPolicies {
                    calendar,
                    ..policies()
                }),
            );
        }
    }
}

#[test]
fn every_explicit_policy_combination_preserves_the_exact_qualification() {
    for mut command in qualifications() {
        select_calendar(&mut command);
        for profile in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
            for source in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
                for calendar in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
                    preserves(
                        command.clone(),
                        Some(TrackingPolicies {
                            profile,
                            source,
                            calendar,
                        }),
                    );
                }
            }
        }
    }
}

#[test]
fn a_known_source_without_a_calendar_preserves_both_declared_policies() {
    for command in qualifications() {
        for profile in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
            for source in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
                preserves(
                    command.clone(),
                    Some(TrackingPolicies {
                        profile,
                        source,
                        calendar: TrackingPolicy::Undetermined,
                    }),
                );
            }
        }
    }
}

#[test]
fn unknown_source_and_absent_calendar_keep_their_explicit_undetermined_policies() {
    for mut command in qualifications() {
        unknown_source(&mut command);
        for profile in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
            preserves(
                command.clone(),
                Some(TrackingPolicies {
                    profile,
                    source: TrackingPolicy::Undetermined,
                    calendar: TrackingPolicy::Undetermined,
                }),
            );
        }
    }
}

#[test]
fn unknown_source_does_not_erase_the_independent_calendar_selection_or_policy() {
    for mut command in qualifications() {
        unknown_source(&mut command);
        select_calendar(&mut command);
        for calendar in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
            preserves(
                command.clone(),
                Some(TrackingPolicies {
                    profile: TrackingPolicy::Follow,
                    source: TrackingPolicy::Undetermined,
                    calendar,
                }),
            );
        }
    }
}

#[test]
fn attention_and_retirement_preserve_the_command_and_never_accept_new_policies() {
    let (register, _) = fixture();
    for change in [
        DeadlineChange::SetAttention {
            expected_revision: DeadlineRevision::new(3).unwrap(),
            attention: attention(),
            reason: text("Declared filing with its original evidence"),
        },
        DeadlineChange::Retire {
            expected_revision: DeadlineRevision::new(3).unwrap(),
            reason: text("Retain the historical calculation after retirement"),
        },
    ] {
        let command = DeadlineCommand {
            operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(103)),
            deadline_id: register.deadline_id,
            change,
        };
        preserves(command.clone(), None);
        rejects(command.clone(), Some(policies()));
        for policy in [
            TrackingPolicy::Undetermined,
            TrackingPolicy::Fixed,
            TrackingPolicy::Follow,
        ] {
            rejects(
                command.clone(),
                Some(TrackingPolicies {
                    profile: policy,
                    source: policy,
                    calendar: policy,
                }),
            );
        }
    }
}
