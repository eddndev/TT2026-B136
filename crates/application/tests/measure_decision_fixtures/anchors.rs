use super::*;
use application::hearings::*;
use uuid::Uuid;

fn ordinary_initial() -> HearingDetail {
    let fixture = Fixture::single();
    let material = fixture.material.context.material();
    let values = HearingValues::new(HearingValuesInput {
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(at().replace_nanosecond(0).unwrap()).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Initial court").unwrap(),
        note: None,
        participants: vec![],
        conviction_basis: None,
    })
    .unwrap();
    let context = HearingContextExpectation {
        case_revision: material.administration.revision,
        stage_revision: material.stage.stage_revision(),
    };
    let command = HearingCommand {
        operation_id: HearingOperationId::from_uuid(Uuid::from_u128(120)),
        hearing_id: HearingId::from_uuid(Uuid::from_u128(121)),
        change: HearingChange::Schedule {
            context,
            values: values.clone(),
        },
    };
    let values_digest = hearing_values_digest(&Hasher, &values);
    HearingDetail {
        snapshot: HearingSnapshot {
            case_id: fixture.case_id,
            id: command.hearing_id,
            revision: HearingRevision::initial(),
            values,
            values_digest,
            status: HearingStatus::Scheduled,
            reason: None,
            receipt: HearingReceipt {
                operation_id: command.operation_id,
                action: HearingAction::Schedule,
                expected_revision: 0,
                expected_context: Some(context),
                submission_digest: hearing_submission_digest(
                    &Hasher,
                    fixture.actor.id,
                    fixture.case_id,
                    &command,
                    values_digest,
                ),
            },
            scheduling_context: HearingSchedulingContext {
                administration_revision: material.administration.revision,
                administration_digest: material.administration.values_digest,
                stage_revision: material.stage.stage_revision(),
                stage: material.stage.stage(),
                stage_digest: None,
            },
            recorded_administration_revision: material.administration.revision,
            recorded_administration_digest: material.administration.values_digest,
            recorded_at: at(),
            recorded_by: material.administration.changed_by.clone(),
        },
        participants: vec![],
        support: None,
    }
}

fn anchors() -> Vec<(MeasureDecisionAnchorRef, MeasureDecisionAnchorMaterial)> {
    let initial = ordinary_initial();
    let precautionary = crate::precautionary_receipt_support::scheduled();
    vec![
        (
            MeasureDecisionAnchorRef::Initial {
                hearing_id: initial.snapshot.id,
                revision: initial.snapshot.revision,
                values_digest: initial.snapshot.values_digest,
                submission_digest: initial.snapshot.receipt.submission_digest,
            },
            MeasureDecisionAnchorMaterial::Initial(Box::new(initial)),
        ),
        (
            MeasureDecisionAnchorRef::Precautionary {
                hearing_id: precautionary.review.command.hearing_id,
                revision: precautionary.review.result_revision,
                capture_digest: precautionary.capture_digest,
            },
            MeasureDecisionAnchorMaterial::Precautionary(Box::new(precautionary)),
        ),
    ]
}

#[test]
fn anchor_references_and_material_are_explicitly_rejected_in_either_family() {
    for (reference, material) in anchors() {
        for selection in 1..=3 {
            for mut fixture in [Fixture::single(), Fixture::no_change()] {
                if selection & 1 != 0 {
                    fixture.command.anchor = Some(reference.clone());
                    assert!(measure_decision_submission_bytes(
                        &fixture.actor,
                        fixture.case_id,
                        &fixture.command
                    )
                    .is_err());
                }
                if selection & 2 != 0 {
                    fixture.material.anchor = Some(material.clone());
                }
                assert!(fixture.prepare().is_err());
            }
        }
    }
}

#[test]
fn public_groups_cannot_add_an_unvalidated_anchor_after_checked_preparation() {
    for (reference, material) in anchors() {
        let mut group = Fixture::single().capture();
        group.review.command.anchor = Some(reference);
        group.review.material.anchor = Some(material.clone());
        group.decision.anchor = Some(material);
        assert_invalid(&group);
        assert!(measure_decision_group_bytes(&group).is_err());
    }
}
