use application::case_stages::CaseStageRevision;
use application::cases::CaseRevision;
use domain::cases::CaseId;
use domain::crypto::Sha256Digest;
use domain::identity::{Role, UserId};
use uuid::Uuid;

use crate::correction_support::*;

fn instruction(fixture: &CorrectionFixture) -> Vec<u8> {
    measure_administrative_submission_bytes(&fixture.actor, fixture.case_id, &fixture.command)
        .unwrap()
}

#[test]
fn submission_bytes_bind_actor_scope_operation_target_context_reason_and_correction_fields() {
    let baseline = CorrectionFixture::initial();
    let expected = instruction(&baseline);
    for mutation in 0..15 {
        let mut fixture = baseline.clone();
        let target = fixture.command.target;
        match mutation {
            0 => fixture.actor.id = UserId::from_uuid(Uuid::from_u128(901)),
            1 => fixture.actor.email = "other@example.test".into(),
            2 => fixture.actor.role = Role::Owner,
            3 => fixture.case_id = CaseId::from_uuid(Uuid::from_u128(2)),
            4 => {
                fixture.command.operation_id =
                    MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(501))
            }
            5 => {
                fixture.command.target =
                    PrecautionaryMeasureRef::new(id(71), target.revision(), target.digest())
            }
            6 => {
                fixture.command.target = PrecautionaryMeasureRef::new(
                    target.id(),
                    MeasureRevision::new(2).unwrap(),
                    target.digest(),
                )
            }
            7 => {
                fixture.command.target = PrecautionaryMeasureRef::new(
                    target.id(),
                    target.revision(),
                    Sha256Digest::from_array([99; 32]),
                )
            }
            8 => fixture.command.context.administration_revision = CaseRevision::new(2).unwrap(),
            9 => fixture.command.context.stage_revision = CaseStageRevision::new(2).unwrap(),
            10 => fixture.command.context.context_digest = Sha256Digest::from_array([99; 32]),
            11 => fixture.command.reason = note("Another transcription explanation"),
            _ => {
                let MeasureAdministrativeAction::Correct(values) = &fixture.command.action else {
                    panic!("correction fixture expected");
                };
                let conditions = if mutation == 12 {
                    note("Other conditions")
                } else {
                    values.conditions().clone()
                };
                let validity = if mutation == 13 {
                    MeasureValidity::new(
                        values.validity().start().clone(),
                        note("Other duration"),
                        values.validity().end().cloned(),
                    )
                    .unwrap()
                } else {
                    values.validity().clone()
                };
                let supervision = if mutation == 14 {
                    note("Other supervision declaration")
                } else {
                    values.supervision_text().clone()
                };
                fixture.command.action = MeasureAdministrativeAction::Correct(
                    MeasureCorrectionValues::new(conditions, validity, supervision),
                );
            }
        }
        assert_ne!(
            instruction(&fixture),
            expected,
            "unbound instruction field {mutation}"
        );
    }
}

#[test]
fn submission_framing_retains_all_four_role_tags_without_granting_recording_authority() {
    let baseline = CorrectionFixture::initial();
    let mut frames = std::collections::HashSet::new();
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let mut fixture = baseline.clone();
        fixture.actor.role = role;
        frames.insert(instruction(&fixture));
    }
    assert_eq!(frames.len(), 4);
}
