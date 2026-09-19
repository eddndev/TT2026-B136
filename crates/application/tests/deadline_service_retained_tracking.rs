#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_service_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    deadline_reevaluation::PredecessorReceipt,
    deadline_tracking::{DeadlineReviewState, TrackingPolicy},
    deadlines::*,
};
use deadline_service_support::*;
use deadline_technical_support as technical;
use domain::identity::Role;

fn pending() -> DeadlineDetail {
    let base = technical::attention(&technical::accepted(TrackingPolicy::Follow));
    let heads = technical::source_heads(&base, 2, false);
    let command = technical::event_command(technical::source_event(&heads, 2));
    technical::revision(&base, command, heads)
}

fn confirm(
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
) -> (DeadlineDraft, DeadlineDetail) {
    let mut store = MockStore::new();
    let expected_command = command.clone();
    store
        .expect_prepare()
        .times(2)
        .returning(move |actor, case, command| {
            assert_eq!(actor, owner());
            assert_eq!(case, case_id());
            assert_eq!(command, &expected_command);
            Ok(preparation.clone())
        });
    store
        .expect_commit()
        .times(1)
        .return_once(|actor, prepared| {
            assert_eq!(actor, owner());
            Ok(tracked_detail(&prepared))
        });
    let (workflow, _) = service(store, identity(Role::Owner, 4));
    let command = human_command(command);
    let draft = workflow
        .prepare("session", case_id(), command.clone())
        .unwrap();
    let committed = workflow
        .submit("session", case_id(), command, draft.submission_digest)
        .unwrap();
    (draft, committed)
}

#[test]
fn attention_and_retirement_preserve_pending_or_undeclared_history() {
    for base in [captured(), pending()] {
        for retirement in [false, true] {
            let change = if retirement {
                DeadlineChange::Retire {
                    expected_revision: base.revision,
                    reason: evaluation::text("Retain the original evidence after retirement"),
                }
            } else {
                DeadlineChange::SetAttention {
                    expected_revision: base.revision,
                    attention: attention(),
                    reason: evaluation::text("Declare filing without reviewing source changes"),
                }
            };
            let (command, preparation) = followup(&base, change);
            assert!(preparation.resolved.is_none());
            let (draft, committed) = confirm(command, preparation);
            assert_eq!(committed.calculation, base.calculation);
            assert_eq!(committed.definition, base.definition);
            assert_eq!(committed.responsible, base.responsible);
            assert_eq!(committed.review_state(), base.review_state());
            assert_eq!(committed.operational_due_at(), None);
            assert_eq!(committed.recorded_by, draft.author);
            assert_eq!(committed.tracking.as_ref(), Some(&draft.tracking));
            if let Some(tracking) = &base.tracking {
                assert_eq!(&draft.tracking, tracking);
            } else {
                assert_eq!(
                    draft.tracking.review.state(),
                    DeadlineReviewState::LegacyUndeclared
                );
                assert_eq!(
                    draft.tracking.policies.profile,
                    TrackingPolicy::Undetermined
                );
                assert_eq!(draft.tracking.policies.source, TrackingPolicy::Undetermined);
                assert_eq!(
                    draft.tracking.policies.calendar,
                    TrackingPolicy::Undetermined
                );
            }
            let DeadlineReceiptVersion::Tracked(metadata) = &committed.receipt.version else {
                panic!("human followup must use tracked receipts");
            };
            assert_eq!(metadata.cause, None);
            assert_eq!(
                metadata.predecessor,
                Some(PredecessorReceipt {
                    submission_digest: base.receipt.submission_digest,
                    capture_digest: base.receipt.capture_digest,
                })
            );
            assert_eq!(
                committed.attention,
                if retirement {
                    base.attention.clone()
                } else {
                    attention()
                }
            );
            deadline_receipt_matches(hasher().as_ref(), &base).unwrap();
            deadline_receipt_matches(hasher().as_ref(), &committed).unwrap();
        }
    }
}

#[test]
fn explicit_correction_accepts_the_new_source_and_keeps_declared_attention() {
    let base = pending();
    assert_eq!(base.review_state(), DeadlineReviewState::Pending);
    let (_, mut preparation) = fixture();
    let mut material = technical::source_heads(&base, 2, false).material;
    material.source = material.source_head.clone();
    let mut definition = base.definition.clone();
    definition.input.selection =
        technical::inputs::request(material.source.as_ref().unwrap()).trigger;
    preparation.resolved.as_mut().unwrap().material = material;
    preparation.base = Some(base.clone());
    let command = DeadlineCommand {
        deadline_id: base.id,
        operation_id: DeadlineOperationId::new(),
        change: DeadlineChange::Correct {
            expected_revision: base.revision,
            definition,
            reason: evaluation::text("Qualify the newly selected exact source"),
        },
    };
    let (draft, committed) = confirm(command, preparation);
    assert_eq!(draft.tracking.review.state(), DeadlineReviewState::Accepted);
    assert!(draft.tracking.review.reasons().is_empty());
    assert_eq!(committed.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(committed.attention, base.attention);
    assert_eq!(committed.responsible, base.responsible);
    assert_ne!(
        committed.calculation.material.source,
        base.calculation.material.source
    );
    assert_eq!(
        committed.calculation.material.source,
        committed.calculation.material.source_head
    );
    assert!(matches!(
        committed.recorded_by,
        DeadlineActorSnapshot::User { .. }
    ));
    assert!(matches!(
        base.recorded_by,
        DeadlineActorSnapshot::Technical { .. }
    ));
    deadline_receipt_matches(hasher().as_ref(), &base).unwrap();
    deadline_receipt_matches(hasher().as_ref(), &committed).unwrap();
}
