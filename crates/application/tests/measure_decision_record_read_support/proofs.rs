use super::*;

#[test]
fn mixed_reads_bind_exact_requested_decision_operation_and_case() {
    let expected = v2(10);
    let other = v2(20);
    let actor = reader(Role::Paralegal);
    for kind in [ReadKind::Get, ReadKind::Operation] {
        assert!(read(
            &service(
                successful_store(&actor, &expected, other.clone(), kind),
                identity(&actor)
            ),
            kind,
            &expected
        )
        .is_err());
    }
    let mut returned = expected.clone();
    let MeasureDecisionRecordReceipt::V2(value) = &mut returned else {
        unreachable!()
    };
    value.group.review.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    reject(&expected, &returned);
}

#[test]
fn both_original_families_validate_every_origin_field() {
    for expected in [v1(10), v2(10)] {
        for field in 0..7 {
            let mut returned = expected.clone();
            let origin = match &mut returned {
                MeasureDecisionRecordReceipt::V1(v) => &mut v.origin,
                MeasureDecisionRecordReceipt::V2(v) => &mut v.origin,
            };
            match field {
                0 => origin.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
                1 => {
                    origin.operation_id =
                        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999))
                }
                2 => origin.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(999)),
                3 => origin.submission_digest = Sha256Digest::from_array([99; 32]),
                4 => origin.review_digest = Sha256Digest::from_array([99; 32]),
                5 => origin.decision_digest = Sha256Digest::from_array([99; 32]),
                _ => origin.group_digest = Sha256Digest::from_array([99; 32]),
            }
            reject(&expected, &returned);
        }
    }
}

#[test]
fn mixed_reads_require_complete_g1_and_c_ancestors_and_all_v2_members() {
    let expected = v2(10);
    for mutation in 0..9 {
        let mut returned = expected.clone();
        let MeasureDecisionRecordReceipt::V2(value) = &mut returned else {
            unreachable!()
        };
        match mutation {
            0 => value.record_history.records.judicial.groups.clear(),
            1 => value.record_history.records.administrative.clear(),
            2 => value
                .record_history
                .records
                .administrative
                .push(value.record_history.records.administrative[0].clone()),
            3 => value.record_history.records.judicial.groups[0]
                .capture
                .measures
                .clear(),
            4 => value.record_history.records.administrative[0]
                .capture
                .records
                .clear(),
            5 => value.group.measures.clear(),
            6 => {
                value.group.measures[0].result.values =
                    root_fixture(99).capture().measures[0].result.values.clone()
            }
            7 => {
                value.group.measures[0]
                    .result
                    .sources
                    .subject
                    .changed_by
                    .email = "invented@example.test".into()
            }
            _ => value.group.review.material.predecessors.clear(),
        }
        reject(&expected, &returned);
    }
}

#[test]
fn original_v2_reads_reject_missing_or_extra_v2_ancestry_and_preserve_transport_order() {
    let MeasureDecisionRecordReceipt::V2(first) = v2(10) else {
        unreachable!()
    };
    let next = FixtureV2::next(&first.group, &first.record_history, 20_000);
    let expected = from_v2(&next);
    for mutation in 0..3 {
        let mut returned = expected.clone();
        let MeasureDecisionRecordReceipt::V2(value) = &mut returned else {
            unreachable!()
        };
        match mutation {
            0 => value.record_history.decisions.clear(),
            1 => value.record_history.decisions[0].capture.measures.clear(),
            _ => value
                .record_history
                .decisions
                .push(value.record_history.decisions[0].clone()),
        }
        reject(&expected, &returned);
    }
    let third = FixtureV2::next(&next.capture(), &next.history, 30_000);
    let mut reordered = from_v2(&third);
    let MeasureDecisionRecordReceipt::V2(value) = &mut reordered else {
        unreachable!()
    };
    value.record_history.decisions.reverse();
    let actor = reader(Role::Owner);
    for kind in READS {
        assert_eq!(
            read(
                &service(
                    successful_store(&actor, &reordered, reordered.clone(), kind),
                    identity(&actor)
                ),
                kind,
                &reordered
            )
            .unwrap(),
            vec![reordered.clone()]
        );
    }
}
