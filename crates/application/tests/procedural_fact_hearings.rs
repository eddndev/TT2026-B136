#[path = "procedural_fact_hearing_support/mod.rs"]
mod support;
use application::{hearing_results::*, procedural_facts::*};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
use support::*;
use uuid::Uuid;

#[test]
fn empty_selection_needs_no_material_or_hash_work() {
    let hasher = Hasher::default();
    assert!(
        resolve_fact_hearings(&hasher, case_id(), &selection(&[]), &[])
            .unwrap()
            .is_empty()
    );
    assert!(hasher.seen.borrow().is_empty());
}

#[test]
fn exact_source_projects_declared_values_both_digests_and_agreement() {
    let source = source(1, 2, 1, false, &[4]);
    let selected = reference(1, 2, 1, Some(4));
    let hasher = Hasher::new(&[&source]);
    let result = resolve_fact_hearings(
        &hasher,
        case_id(),
        &selection(&[selected]),
        std::slice::from_ref(&source),
    )
    .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].snapshot,
        FactHearingSourceSnapshot {
            case_id: case_id(),
            reference: selected,
            values_digest: source.values_digest,
            submission_digest: source.receipt.submission_digest,
            status: source.status,
        }
    );
    assert_ne!(source.values_digest, source.receipt.submission_digest);
    assert_eq!(
        result[0].view,
        FactHearingView {
            reference: selected,
            occurrence: source.values.occurrence(),
            event_time: source.values.event_time(),
            summary: source.values.summary().clone(),
            agreement: source.values.agreements().first().cloned(),
        }
    );
    let seen = hasher.seen.borrow();
    assert!(seen.contains(&source.values.canonical_bytes()));
    assert!(seen.contains(&receipt_bytes(&source)));
}

#[test]
fn two_agreements_share_one_exact_snapshot_and_keep_individual_views() {
    let source = source(1, 2, 3, false, &[4, 5]);
    let first = reference(1, 2, 3, Some(4));
    let second = reference(1, 2, 3, Some(5));
    let result = resolve_fact_hearings(
        &Hasher::new(&[&source]),
        case_id(),
        &selection(&[second, first]),
        std::slice::from_ref(&source),
    )
    .unwrap();
    assert_eq!(result.len(), 2);
    for (projection, (reference, agreement)) in result.iter().zip([
        (first, &source.values.agreements()[0]),
        (second, &source.values.agreements()[1]),
    ]) {
        assert_eq!(projection.snapshot.reference, reference);
        assert_eq!(projection.view.reference, reference);
        assert_eq!(projection.view.agreement.as_ref(), Some(agreement));
        assert_eq!(projection.snapshot.values_digest, source.values_digest);
    }
}

#[test]
fn absent_agreement_is_distinct_from_the_zero_uuid_agreement() {
    let source = source(1, 2, 1, false, &[0]);
    let absent = reference(1, 2, 1, None);
    let zero = reference(1, 2, 1, Some(0));
    let result = resolve_fact_hearings(
        &Hasher::new(&[&source]),
        case_id(),
        &selection(&[zero, absent]),
        &[source],
    )
    .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].view.reference, absent);
    assert_eq!(result[0].view.agreement, None);
    assert_eq!(result[1].view.reference, zero);
    assert_eq!(
        result[1].view.agreement.as_ref().unwrap().id().as_uuid(),
        Uuid::nil()
    );
}

#[test]
fn two_sources_are_projected_in_selection_order_from_unordered_material() {
    let first = source(1, 9, 1, false, &[4]);
    let second = source(2, 3, 2, true, &[5]);
    let a = reference(1, 9, 1, Some(4));
    let b = reference(2, 3, 2, Some(5));
    let hasher = Hasher::new(&[&first, &second]);
    let result = resolve_fact_hearings(
        &hasher,
        case_id(),
        &selection(&[b, a]),
        &[second.clone(), first.clone()],
    )
    .unwrap();
    assert_eq!(result.len(), 2);
    for (projection, (reference, snapshot)) in result.iter().zip([(a, first), (b, second)]) {
        assert_eq!(projection.snapshot.reference, reference);
        assert_eq!(projection.snapshot.status, snapshot.status);
        assert_eq!(projection.view.summary, *snapshot.values.summary());
        assert_eq!(
            projection.view.agreement.as_ref(),
            snapshot.values.agreements().first()
        );
    }
}

#[test]
fn two_revisions_of_one_result_remain_distinct_historical_sources() {
    let first = source(1, 2, 1, false, &[4]);
    let second = source(1, 2, 2, false, &[4]);
    let a = reference(1, 2, 1, Some(4));
    let b = reference(1, 2, 2, Some(4));
    let result = resolve_fact_hearings(
        &Hasher::new(&[&first, &second]),
        case_id(),
        &selection(&[b, a]),
        &[second, first],
    )
    .unwrap();
    assert_eq!(result[0].snapshot.reference, a);
    assert_eq!(result[1].snapshot.reference, b);
    assert_ne!(result[0].view.agreement, result[1].view.agreement);
}

#[test]
fn repeated_exact_selection_needs_one_material_and_one_projection() {
    let source = source(1, 2, 1, false, &[4]);
    let reference = reference(1, 2, 1, Some(4));
    let result = resolve_fact_hearings(
        &Hasher::new(&[&source]),
        case_id(),
        &selection(&[reference, reference]),
        &[source],
    )
    .unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn missing_extra_and_more_than_two_snapshots_are_rejected() {
    let a = source(1, 2, 1, false, &[]);
    let b = source(2, 3, 1, false, &[]);
    let c = source(3, 4, 1, false, &[]);
    let one = selection(&[reference(1, 2, 1, None)]);
    let two = selection(&[reference(1, 2, 1, None), reference(2, 3, 1, None)]);
    let hasher = Hasher::new(&[&a, &b, &c]);
    assert_inconsistent(resolve_fact_hearings(&hasher, case_id(), &one, &[]));
    assert_inconsistent(resolve_fact_hearings(
        &hasher,
        case_id(),
        &selection(&[]),
        std::slice::from_ref(&a),
    ));
    assert_inconsistent(resolve_fact_hearings(
        &hasher,
        case_id(),
        &two,
        std::slice::from_ref(&a),
    ));
    assert_inconsistent(resolve_fact_hearings(
        &hasher,
        case_id(),
        &one,
        &[a.clone(), b.clone()],
    ));
    assert_inconsistent(resolve_fact_hearings(&hasher, case_id(), &two, &[a, b, c]));
}

#[test]
fn duplicate_material_does_not_replace_a_missing_result_even_at_equal_count() {
    let source = source(1, 2, 1, false, &[]);
    assert_inconsistent(resolve_fact_hearings(
        &Hasher::new(&[&source]),
        case_id(),
        &selection(&[reference(1, 2, 1, None), reference(2, 3, 1, None)]),
        &[source.clone(), source],
    ));
}

#[test]
fn foreign_case_or_wrong_exact_identity_reject_even_self_consistent_receipts() {
    let mut foreign = source(1, 2, 1, false, &[]);
    foreign.case_id = CaseId::from_uuid(Uuid::from_u128(9));
    for material in [
        foreign,
        source(2, 2, 1, false, &[]),
        source(1, 3, 1, false, &[]),
        source(1, 2, 2, false, &[]),
    ] {
        let hasher = Hasher::new(&[&material]);
        assert!(hearing_result_snapshot_receipt_matches(&hasher, &material).is_ok());
        assert_inconsistent(resolve_fact_hearings(
            &hasher,
            case_id(),
            &selection(&[reference(1, 2, 1, None)]),
            &[material],
        ));
    }
}

#[test]
fn selected_agreement_must_exist_in_that_exact_revision() {
    let source = source(1, 2, 1, false, &[4]);
    let hasher = Hasher::new(&[&source]);
    assert_inconsistent(resolve_fact_hearings(
        &hasher,
        case_id(),
        &selection(&[reference(1, 2, 1, Some(5))]),
        std::slice::from_ref(&source),
    ));
    assert_inconsistent(resolve_fact_hearings(
        &hasher,
        case_id(),
        &selection(&[reference(1, 2, 1, Some(4)), reference(1, 2, 1, Some(5))]),
        &[source],
    ));
}

#[test]
fn changed_values_are_rejected_even_when_the_original_receipt_is_unchanged() {
    let mut source = source(1, 2, 1, false, &[4]);
    let hasher = Hasher::new(&[&source]);
    let mut values = values_input(&source.values);
    values.summary = HearingResultText::new("Altered declaration").unwrap();
    source.values = HearingResultValues::new(values).unwrap();
    assert_inconsistent(resolve_fact_hearings(
        &hasher,
        case_id(),
        &selection(&[reference(1, 2, 1, None)]),
        &[source],
    ));
}

#[test]
fn changed_digest_actor_operation_or_anchor_cannot_keep_the_original_receipt() {
    for mutation in 0..5 {
        let mut source = source(1, 2, 1, false, &[]);
        let hasher = Hasher::new(&[&source]);
        match mutation {
            0 => source.values_digest = Sha256Digest::from_array([0; 32]),
            1 => source.receipt.submission_digest = Sha256Digest::from_array([0; 32]),
            2 => source.recorded_by.id = UserId::from_uuid(Uuid::from_u128(99)),
            3 => source.receipt.operation_id = HearingResultOperationId::from_uuid(Uuid::nil()),
            4 => source.anchor.values_digest = Sha256Digest::from_array([0; 32]),
            _ => unreachable!(),
        }
        assert_inconsistent(resolve_fact_hearings(
            &hasher,
            case_id(),
            &selection(&[reference(1, 2, 1, None)]),
            &[source],
        ));
    }
}

#[test]
fn invalid_status_revision_reason_or_original_capture_is_rejected() {
    for mutation in 0..4 {
        let mut source = source(1, 2, 1, false, &[]);
        let hasher = Hasher::new(&[&source]);
        match mutation {
            0 => source.status = HearingResultStatus::Withdrawn,
            1 => source.receipt.expected_revision = 1,
            2 => source.reason = Some(HearingResultText::new("Unexpected record reason").unwrap()),
            3 => source.recorded_at = OffsetDateTime::from_unix_timestamp(-1).unwrap(),
            _ => unreachable!(),
        }
        assert_inconsistent(resolve_fact_hearings(
            &hasher,
            case_id(),
            &selection(&[reference(1, 2, 1, None)]),
            &[source],
        ));
    }
}

#[test]
fn an_invalid_second_source_rejects_the_entire_projection() {
    let first = source(1, 2, 1, false, &[]);
    let mut second = source(2, 3, 1, false, &[]);
    let hasher = Hasher::new(&[&first, &second]);
    second.receipt.submission_digest = Sha256Digest::from_array([0; 32]);
    assert_inconsistent(resolve_fact_hearings(
        &hasher,
        case_id(),
        &selection(&[reference(1, 2, 1, None), reference(2, 3, 1, None)]),
        &[first, second],
    ));
}
