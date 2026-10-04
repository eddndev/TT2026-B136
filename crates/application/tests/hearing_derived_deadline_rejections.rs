use crate::deadline_support::evaluation::{self, inputs};
use crate::hearing_derived_deadline_support::*;
use application::{
    case_stages::StageSupportSnapshot,
    deadline_profiles::DeadlineProfileScope,
    deadline_tracking::TrackingPolicy,
    deadlines::*,
    documents::{StageDocumentFormat, StageFormatPolicy},
    hearing_results::*,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    deadline_triggers::{TriggerField, TriggerRequirement, TriggerSourceRef},
    hearings::HearingId,
    identity::UserId,
    judicial_calendars::JudicialCalendarClassification,
    procedural_facts::{FactDeclaration, FactResolutionRef, FactRevision, ResolutionId},
};

#[derive(Debug, Clone, Copy)]
enum InvalidBinding {
    ResultCorrection,
    ResultWithdrawal,
    DeadlineCorrection,
    DeadlineRetirement,
    SelectionCase,
    ResultCase,
    Actor,
    SourceFamily,
    ProfileFamily,
    ProfileScope,
    ResultIdentity,
    HearingIdentity,
    ResultRevision,
    MissingAgreement,
    MissingSource,
    Responsible,
    CommandMaterial,
}

#[test]
fn only_a_matching_new_ordinary_result_and_its_configured_deadline_are_accepted() {
    for binding in [
        InvalidBinding::ResultCorrection,
        InvalidBinding::ResultWithdrawal,
        InvalidBinding::DeadlineCorrection,
        InvalidBinding::DeadlineRetirement,
        InvalidBinding::SelectionCase,
        InvalidBinding::ResultCase,
        InvalidBinding::Actor,
        InvalidBinding::SourceFamily,
        InvalidBinding::ProfileFamily,
        InvalidBinding::ProfileScope,
        InvalidBinding::ResultIdentity,
        InvalidBinding::HearingIdentity,
        InvalidBinding::ResultRevision,
        InvalidBinding::MissingAgreement,
        InvalidBinding::MissingSource,
        InvalidBinding::Responsible,
        InvalidBinding::CommandMaterial,
    ] {
        let mut fixture = fixture();
        invalidate(&mut fixture, binding);
        assert!(fixture.prepare().is_err(), "{binding:?}");
    }
}

fn invalidate(fixture: &mut Fixture, binding: InvalidBinding) {
    match binding {
        InvalidBinding::ResultCorrection | InvalidBinding::ResultWithdrawal => {
            let revision = HearingResultRevision::initial();
            let reason = HearingResultText::new("Separate correction or withdrawal").unwrap();
            fixture.command.result.change = match binding {
                InvalidBinding::ResultCorrection => HearingResultChange::Correct {
                    expected_revision: revision,
                    values: fixture.material.result.values.clone(),
                    reason,
                },
                _ => HearingResultChange::Withdraw {
                    expected_revision: revision,
                    reason,
                },
            };
            fixture.material.result.command = fixture.command.result.clone();
            fixture.material.result.result_revision = HearingResultRevision::new(2).unwrap();
        }
        InvalidBinding::DeadlineCorrection => fixture.edit_deadline(|command, _| {
            let DeadlineChange::Register { definition } = &command.change else {
                panic!("registration expected")
            };
            command.change = DeadlineChange::Correct {
                expected_revision: DeadlineRevision::initial(),
                definition: definition.clone(),
                reason: evaluation::text("Separate correction"),
            };
        }),
        InvalidBinding::DeadlineRetirement => {
            let (mut command, _) = fixture.command.deadline.clone().into_parts();
            command.change = DeadlineChange::Retire {
                expected_revision: DeadlineRevision::initial(),
                reason: evaluation::text("Separate retirement"),
            };
            fixture.command.deadline = DeadlineHumanCommand::new(command, None).unwrap();
        }
        InvalidBinding::SelectionCase => {
            fixture.edit_definition(|value| value.input.selection.case_id = CaseId::new())
        }
        InvalidBinding::ResultCase => fixture.material.result.case_id = CaseId::new(),
        InvalidBinding::Actor => fixture.actor.id = UserId::new(),
        InvalidBinding::SourceFamily => fixture.edit_definition(|value| {
            value.input.selection.source =
                FactDeclaration::Known(TriggerSourceRef::Resolution(FactResolutionRef {
                    id: ResolutionId::new(),
                    revision: FactRevision::initial(),
                }));
        }),
        InvalidBinding::ProfileFamily => fixture.edit_profile(|value| {
            value.trigger = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
        }),
        InvalidBinding::ProfileScope => {
            fixture.edit_profile(|value| value.scope = DeadlineProfileScope::Case(CaseId::new()))
        }
        InvalidBinding::ResultIdentity
        | InvalidBinding::HearingIdentity
        | InvalidBinding::ResultRevision
        | InvalidBinding::MissingAgreement => fixture.edit_definition(|value| {
            let FactDeclaration::Known(TriggerSourceRef::HearingResult(source)) =
                &mut value.input.selection.source
            else {
                panic!("hearing source expected")
            };
            match binding {
                InvalidBinding::ResultIdentity => source.result_id = HearingResultId::new(),
                InvalidBinding::HearingIdentity => source.hearing_id = HearingId::new(),
                InvalidBinding::ResultRevision => {
                    source.revision = HearingResultRevision::new(2).unwrap()
                }
                _ => source.agreement_id = Some(agreement(999)),
            }
        }),
        InvalidBinding::MissingSource => fixture.edit_deadline(|command, policies| {
            let DeadlineChange::Register { definition } = &mut command.change else {
                panic!("registration expected")
            };
            definition.input.selection.source =
                FactDeclaration::Unknown(evaluation::text("No source selected"));
            policies.source = TrackingPolicy::Undetermined;
        }),
        InvalidBinding::Responsible => fixture.material.responsible.id = UserId::new(),
        InvalidBinding::CommandMaterial => {
            fixture.material.result.command.operation_id = HearingResultOperationId::new()
        }
    }
}

#[test]
fn corrupt_dependency_receipts_or_mismatched_exact_heads_reject_preparation() {
    for field in [
        "result_values",
        "result_submission",
        "profile_values",
        "profile_submission",
        "calendar_values",
        "calendar_submission",
        "profile_head",
        "calendar_head",
    ] {
        let mut fixture = fixture();
        let forged = Sha256Digest::from_array([7; 32]);
        match field {
            "result_values" => fixture.material.result.values_digest = forged,
            "result_submission" => fixture.material.result.submission_digest = forged,
            "profile_values" => fixture.material.profile.definition_digest = forged,
            "profile_submission" => fixture.material.profile.receipt.submission_digest = forged,
            "calendar_values" => fixture.material.calendar.as_mut().unwrap().values_digest = forged,
            "calendar_submission" => {
                fixture
                    .material
                    .calendar
                    .as_mut()
                    .unwrap()
                    .receipt
                    .submission_digest = forged
            }
            "profile_head" => {
                fixture.material.profile_head.recorded_by.email = "different@example.com".into()
            }
            _ => {
                fixture
                    .material
                    .calendar_head
                    .as_mut()
                    .unwrap()
                    .recorded_by
                    .email = "different@example.com".into()
            }
        }
        assert!(fixture.prepare().is_err(), "{field}");
    }
}

#[test]
fn retired_calendar_cannot_activate_a_new_configured_deadline() {
    let mut fixture = fixture();
    let retired = inputs::calendar(2, true, JudicialCalendarClassification::Countable);
    fixture.edit_definition(|value| {
        value.input.calendar.as_mut().unwrap().revision = retired.revision
    });
    fixture.material.calendar = Some(retired.clone());
    fixture.material.calendar_head = Some(retired);
    assert!(fixture.prepare().is_err());
}

#[test]
fn an_undeclared_support_projection_cannot_be_injected_into_the_result_draft() {
    let mut fixture = fixture();
    fixture.material.result.support = Some(StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::new(),
            version: DocumentVersion::initial(),
        },
        digest: Sha256Digest::from_array([3; 32]),
        name: "Undeclared support".into(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    });
    assert!(fixture.prepare().is_err());
}

#[test]
fn a_stale_result_command_cannot_change_the_original_offset_of_the_same_instant() {
    let mut fixture = fixture();
    let instant = evaluation::date("2026-01-06")
        .date()
        .midnight()
        .assume_utc();
    fixture.edit_values(|value| {
        value.event_time = DeclaredHearingResultTime::instant(instant).unwrap();
    });
    let approved = fixture.prepare().unwrap();
    let HearingResultChange::Record { values, .. } = &mut fixture.material.result.command.change
    else {
        panic!("registration expected")
    };
    let mut altered = crate::hearing_result_support::values_input(values);
    altered.event_time = DeclaredHearingResultTime::instant(
        instant.to_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap()),
    )
    .unwrap();
    assert_eq!(
        values.event_time().instant_value(),
        altered.event_time.instant_value()
    );
    assert_ne!(values.event_time().offset(), altered.event_time.offset());
    *values = HearingResultValues::new(altered).unwrap();
    assert!(fixture.prepare().is_err());
    assert!(approved.require_review(approved.review_digest()).is_ok());
}

#[test]
fn the_same_calendar_revision_cannot_have_two_distinct_capture_offsets() {
    let mut fixture = fixture();
    let selected = fixture.material.calendar.as_ref().unwrap().recorded_at;
    let head = fixture.material.calendar_head.as_mut().unwrap();
    head.recorded_at = selected.to_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap());
    assert_eq!(selected, head.recorded_at);
    assert_ne!(selected.offset(), head.recorded_at.offset());
    assert!(fixture.prepare().is_err());
}

#[test]
fn compound_preparation_requires_both_permissions_and_an_eligible_responsible() {
    use application::ApplicationError;
    use domain::identity::Role;
    for role in [Role::Paralegal, Role::Client] {
        let mut fixture = fixture();
        fixture.actor.role = role;
        assert!(matches!(
            fixture.prepare(),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    for field in ["role", "email"] {
        let mut fixture = fixture();
        if field == "role" {
            fixture.material.responsible.role = Role::Client;
        } else {
            fixture.material.responsible.email = "  ".into();
        }
        assert!(matches!(
            fixture.prepare(),
            Err(ApplicationError::Deadline(
                DeadlineError::ResponsibleUnavailable
            ))
        ));
    }
}
