use crate::hearing_derived_deadline_capture_support::*;
use crate::hearing_derived_deadline_support::*;
use crate::hearing_result_support::hasher;
use application::{
    deadline_evaluations::{DeadlineEvaluationBlock, DeadlineEvaluationRecord},
    deadline_inputs::DeadlineSourceDetail,
    deadlines::{deadline_receipt_matches, DeadlineActorSnapshot, DeadlineReceiptVersion},
    hearing_derived_deadlines::*,
};
use domain::deadline_profiles::DeadlineRuleBlock;
use time::Duration;

#[test]
fn actual_source_capture_keeps_review_but_binds_final_deadline_and_compound_evidence() {
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let mut deadline_captures = Vec::new();
    let mut compound_captures = Vec::new();
    for delay in [1, 2] {
        let recorded_at = crate::case_support::instant() + Duration::hours(delay);
        let source = recorded_source(&fixture, recorded_at);
        let event = source_event(&source);
        let creation =
            finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source.clone(), event)
                .unwrap();
        assert_eq!(creation.draft(), &reviewed);
        assert_eq!(creation.result(), &source);
        assert_eq!(creation.source_event(), event);
        assert!(creation
            .draft()
            .require_review(reviewed.review_digest())
            .is_ok());
        let deadline = creation.deadline();
        assert_eq!(deadline.recorded_at, recorded_at);
        assert_eq!(
            deadline.recorded_at.offset(),
            source.snapshot.recorded_at.offset()
        );
        assert_eq!(
            deadline.recorded_by,
            DeadlineActorSnapshot::User {
                id: fixture.actor.id,
                email: fixture.actor.email.clone(),
            }
        );
        assert_eq!(
            deadline.calculation.result,
            DeadlineEvaluationRecord::capture(reviewed.evaluation()),
        );
        let Some(DeadlineSourceDetail::HearingResult(captured)) =
            &deadline.calculation.material.source
        else {
            panic!("captured result expected")
        };
        assert_eq!(captured.as_ref(), &source);
        let DeadlineReceiptVersion::Tracked(receipt) = &deadline.receipt.version else {
            panic!("tracked receipt required")
        };
        assert_eq!(receipt.predecessor, None);
        assert_eq!(receipt.cause, None);
        deadline_receipt_matches(hasher().as_ref(), deadline).unwrap();
        let bytes = hearing_derived_deadline_capture_bytes(&creation).unwrap();
        assert_eq!(hasher().hash_bytes(&bytes), creation.capture_digest());
        deadline_captures.push(deadline.receipt.capture_digest);
        compound_captures.push(creation.capture_digest());
    }
    assert_ne!(deadline_captures[0], deadline_captures[1]);
    assert_ne!(compound_captures[0], compound_captures[1]);
}

#[test]
fn finalizing_a_blocked_review_retains_its_explicit_reason_and_no_operational_due_time() {
    let mut fixture = fixture();
    fixture.edit_definition(|value| value.input.ordered_quantity = None);
    let reviewed = fixture.prepare().unwrap();
    let source = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::minutes(5),
    );
    let event = source_event(&source);
    let creation =
        finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source, event).unwrap();
    assert_eq!(
        creation.deadline().calculation.result.blocks(),
        &[DeadlineEvaluationBlock::Rule(
            DeadlineRuleBlock::MissingOrderedQuantity
        ),]
    );
    assert_eq!(creation.deadline().calculation.result.due_at(), None);
    assert_eq!(creation.deadline().operational_due_at(), None);
    deadline_receipt_matches(hasher().as_ref(), creation.deadline()).unwrap();
}

#[test]
fn identical_final_material_has_identical_canonical_capture_bytes() {
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let source = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::minutes(5),
    );
    let event = source_event(&source);
    let first =
        finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source.clone(), event)
            .unwrap();
    let repeated =
        finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source, event).unwrap();
    assert_eq!(first.deadline(), repeated.deadline());
    assert_eq!(first.capture_digest(), repeated.capture_digest());
    assert_eq!(
        hearing_derived_deadline_capture_bytes(&first).unwrap(),
        hearing_derived_deadline_capture_bytes(&repeated).unwrap(),
    );
}
