#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_input_support;
use application::{
    deadline_inputs::*, hearing_results::*, judicial_calendars::*, procedural_facts::*,
};
use deadline_input_support::*;
use domain::{cases::CaseId, crypto::Sha256Digest, deadline_arithmetic::*, deadline_triggers::*};
use std::num::NonZeroU32;

fn sources(revision: u32, retired: bool, at: &str) -> Vec<DeadlineSourceDetail> {
    vec![
        DeadlineSourceDetail::Fact(Box::new(resolution(revision, retired, at))),
        DeadlineSourceDetail::Fact(Box::new(notification(revision, retired, revision, at))),
        DeadlineSourceDetail::HearingResult(Box::new(hearing(revision, retired, at, &[]))),
    ]
}
fn validate_source(source: &DeadlineSourceDetail) {
    match source {
        DeadlineSourceDetail::Fact(detail) => {
            fact_receipt_matches(hasher().as_ref(), detail).unwrap()
        }
        DeadlineSourceDetail::HearingResult(detail) => {
            hearing_result_receipt_matches(hasher().as_ref(), detail).unwrap()
        }
    }
}
#[test]
fn newer_withdrawn_heads_preserve_each_selected_historical_anchor() {
    for (exact, head) in
        sources(1, false, "2026-01-06")
            .into_iter()
            .zip(sources(3, true, "2026-01-09"))
    {
        let request = request(&exact);
        let mut material = material(exact);
        material.source_head = Some(head);
        let before = material.clone();
        let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
        assert_eq!(
            result.arithmetic().unwrap().outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: "2026-01-06".parse().unwrap()
            }
        );
        assert_eq!(material, before);
        assert_eq!(result.trigger().selection(), &request.trigger);
    }
}
#[test]
fn notification_new_head_can_select_a_new_revision_of_the_same_parent() {
    let exact = DeadlineSourceDetail::Fact(Box::new(notification(1, false, 1, "2026-01-06")));
    let head = notification(2, false, 2, "2026-01-09");
    let request = request(&exact);
    let mut material = material(exact);
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(head)));
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    let TriggerSourceRef::Notification { resolution, .. } =
        result.trigger().source().unwrap().reference
    else {
        unreachable!()
    };
    assert_eq!(resolution.revision, FactRevision::initial());
    assert_eq!(result.arithmetic().unwrap().anchor(), date("2026-01-06"));
}
#[test]
fn agreement_removed_from_head_remains_selected_from_exact_result_even_when_uuid_zero() {
    let exact =
        DeadlineSourceDetail::HearingResult(Box::new(hearing(1, false, "2026-01-06", &[0])));
    let mut request = request(&exact);
    let FactDeclaration::Known(TriggerSourceRef::HearingResult(reference)) =
        &mut request.trigger.source
    else {
        unreachable!()
    };
    reference.agreement_id = Some(HearingResultAgreementId::from_uuid(uuid::Uuid::nil()));
    let mut material = material(exact);
    material.source_head = Some(DeadlineSourceDetail::HearingResult(Box::new(hearing(
        2,
        false,
        "2026-01-09",
        &[],
    ))));
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    assert_eq!(
        result
            .trigger()
            .source()
            .unwrap()
            .agreement
            .as_ref()
            .unwrap()
            .id(),
        HearingResultAgreementId::from_uuid(uuid::Uuid::nil())
    );
    assert_eq!(
        result.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: "2026-01-06".parse().unwrap()
        }
    );
}
#[test]
fn an_older_head_is_rejected_for_every_source_family() {
    for (exact, head) in
        sources(2, false, "2026-01-06")
            .into_iter()
            .zip(sources(1, false, "2026-01-06"))
    {
        let request = request(&exact);
        let mut material = material(exact);
        material.source_head = Some(head);
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &request,
            &material,
        ));
    }
}
#[test]
fn same_revision_requires_complete_detail_equality_even_when_both_receipts_match() {
    for exact in sources(1, false, "2026-01-06") {
        let request = request(&exact);
        let mut head = exact.clone();
        match &mut head {
            DeadlineSourceDetail::Fact(detail) => {
                metadata_mut(detail).recorded_by.email = "changed@example.test".into()
            }
            DeadlineSourceDetail::HearingResult(detail) => {
                detail.snapshot.recorded_by.email = "changed@example.test".into()
            }
        }
        validate_source(&head);
        let mut material = material(exact);
        material.source_head = Some(head);
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &request,
            &material,
        ));
    }
}
#[test]
fn head_receipt_corruption_is_not_hidden_by_a_valid_historical_source() {
    for (exact, head) in
        sources(1, false, "2026-01-06")
            .into_iter()
            .zip(sources(2, false, "2026-01-09"))
    {
        for field in 0..3 {
            let request = request(&exact);
            let mut corrupt = head.clone();
            let bad = Sha256Digest::from_array([255; 32]);
            match &mut corrupt {
                DeadlineSourceDetail::Fact(detail) => {
                    let metadata = metadata_mut(detail);
                    match field {
                        0 => metadata.values_digest = bad,
                        1 => metadata.receipt.sources_digest = bad,
                        _ => metadata.receipt.submission_digest = bad,
                    }
                }
                DeadlineSourceDetail::HearingResult(detail) => match field {
                    0 => detail.snapshot.values_digest = bad,
                    1 => detail.snapshot.receipt.submission_digest = bad,
                    _ => detail.anchor.reference.values_digest = bad,
                },
            }
            let mut material = material(exact.clone());
            material.source_head = Some(corrupt);
            inconsistent(check_deadline_inputs(
                hasher().as_ref(),
                &request,
                &material,
            ));
        }
    }
}

#[path = "deadline_input_support/calendar_heads.rs"]
mod calendar_heads;

#[test]
fn valid_head_from_a_different_family_is_rejected() {
    let families = sources(1, false, "2026-01-06");
    for index in 0..families.len() {
        let exact = families[index].clone();
        let request = request(&exact);
        let mut material = material(exact);
        material.source_head = Some(families[(index + 1) % families.len()].clone());
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &request,
            &material,
        ));
    }
}

#[path = "deadline_input_support/head_identity_cases.rs"]
mod head_identity_cases;
