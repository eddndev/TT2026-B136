use super::*;

#[path = "clock_bounds_tests.rs"]
mod clocks_and_bounds;
#[path = "page_integrity_tests.rs"]
mod pages;

fn dependent(
    previous: &MeasureDecisionStoredOperation,
    serial: u128,
) -> MeasureDecisionStoredOperation {
    let mut next = crate::effect_support::LaterFixture::next(
        &previous.group,
        &previous.measure_history,
        serial,
    );
    next.request.command.decision_id = id(serial);
    next.request.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(3000 + serial));
    let evidence = next.evidence.clone();
    let group = next
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, previous.group.recorded_at + Duration::seconds(1))
        .unwrap();
    stored(group, evidence)
}

fn refresh_claims(operation: &mut MeasureDecisionStoredOperation) {
    crate::measure_decision_fixtures::refresh_digests(&mut operation.group);
    operation.origin.submission_digest = operation.group.review.submission_digest;
    operation.origin.review_digest = operation.group.review.review_digest;
    operation.origin.decision_digest = operation.group.decision.capture_digest;
    operation.origin.group_digest = operation.group.capture_digest;
}

fn reject_all_reads(
    expected: &MeasureDecisionStoredOperation,
    returned: &MeasureDecisionStoredOperation,
) {
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, expected, returned.clone(), kind);
        assert!(read(&service(store, identity(&actor)), kind, expected).is_err());
    }
}

#[test]
fn each_read_checks_every_exact_origin_field() {
    let saved = operation(10);
    for mutation in 0..7 {
        let mut returned = saved.clone();
        match mutation {
            0 => returned.origin.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
            1 => {
                returned.origin.operation_id =
                    MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999))
            }
            2 => returned.origin.decision_id = id(999),
            3 => returned.origin.submission_digest = Sha256Digest::from_array([99; 32]),
            4 => returned.origin.review_digest = Sha256Digest::from_array([99; 32]),
            5 => returned.origin.decision_digest = Sha256Digest::from_array([99; 32]),
            _ => returned.origin.group_digest = Sha256Digest::from_array([99; 32]),
        }
        reject_all_reads(&saved, &returned);
    }
}

#[test]
fn every_read_reconstructs_the_complete_decision_and_all_group_members() {
    let saved = operation(10);
    for mutation in 0..5 {
        let mut returned = saved.clone();
        match mutation {
            0 => returned.group.decision.capture_digest = Sha256Digest::from_array([99; 32]),
            1 => returned.group.measures[0].capture_digest = Sha256Digest::from_array([99; 32]),
            2 => returned.group.measures.clear(),
            3 => returned
                .group
                .measures
                .push(returned.group.measures[0].clone()),
            _ => {
                returned.group.measures[0]
                    .result
                    .projection
                    .subject
                    .display_name = "Invented captured subject".into()
            }
        }
        if mutation >= 2 {
            refresh_claims(&mut returned);
        }
        reject_all_reads(&saved, &returned);
    }
}

#[test]
fn every_read_requires_the_complete_exact_measure_ancestor_closure() {
    let saved = dependent(&operation(100), 10);
    for mutation in 0..5 {
        let mut returned = saved.clone();
        let groups = &mut returned.measure_history.groups;
        match mutation {
            0 => groups.clear(),
            1 => groups[0].origin.review_digest = Sha256Digest::from_array([99; 32]),
            2 => groups[0].capture.measures.clear(),
            3 => groups.push(groups[0].clone()),
            _ => {
                let extra = operation(200);
                groups.push(MeasureGroupEvidence {
                    origin: extra.origin,
                    capture: extra.group,
                });
            }
        }
        reject_all_reads(&saved, &returned);
    }
}

#[test]
fn reads_reject_an_ancestor_with_rehashed_invented_material() {
    let saved = dependent(&operation(100), 10);
    let mut returned = saved.clone();
    let ancestor = &mut returned.measure_history.groups[0];
    ancestor.capture.measures[0]
        .result
        .projection
        .subject
        .display_name = "Invented ancestor subject".into();
    crate::measure_decision_fixtures::refresh_digests(&mut ancestor.capture);
    ancestor.origin.group_digest = ancestor.capture.capture_digest;
    returned.group.review.material.predecessors[0]
        .owner
        .group_digest = ancestor.capture.capture_digest;
    returned.group.review.material.predecessors[0].capture = ancestor.capture.measures[0].clone();
    refresh_claims(&mut returned);
    reject_all_reads(&saved, &returned);
}

#[test]
fn reads_validate_atomic_substitution_links_even_after_the_group_is_rehashed() {
    let initial = operation(100);
    let next = crate::effect_support::substitution(&initial.group, &[90, 100]);
    let evidence = next.evidence.clone();
    let saved = stored(next.capture(), evidence);
    let mut returned = saved.clone();
    returned.group.substitutions[0].successors.pop();
    refresh_claims(&mut returned);
    reject_all_reads(&saved, &returned);
}
