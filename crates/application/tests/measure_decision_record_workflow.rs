#[allow(dead_code)]
mod case_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_correction_capture_support/mod.rs"]
mod correction_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
mod measure_decision_record_workflow_support;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[path = "document_format_support/mod.rs"]
mod observed_crypto;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

use measure_decision_record_workflow_support::*;

#[test]
fn record_workflow_prepares_genuine_v2_from_the_exact_corrected_record() {
    let fixture = Fixture::corrected();
    let expected = fixture.review();
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    let review = harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .unwrap();
    let MeasureDecisionRecordReview::V2(review) = review else {
        panic!("fresh decision-record preparation must use the V2 family")
    };
    assert_eq!(*review, expected);
    assert!(matches!(
        &review.material.predecessors[0],
        OwnedMeasureRecord::Administrative { .. }
    ));
    assert_eq!(harness.validator.calls(), 1);
    assert!(harness.events().contains(&"open"));
    assert!(harness.events().contains(&"hash"));
}

#[test]
fn record_workflow_submits_actual_v2_group_with_unchanged_c_and_g1_ancestry() {
    let fixture = Fixture::corrected();
    let expected = fixture.operation(now());
    let reviewed = expected.group.review.clone();
    let material = fixture.material.clone();
    let actor = fixture.actor.clone();
    let case_id = fixture.case_id;
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |principal, case, prepared| {
            assert_eq!(principal, &actor);
            assert_eq!(case, case_id);
            assert_eq!(prepared.actor(), &actor);
            assert_eq!(prepared.review(), &reviewed);
            assert_eq!(prepared.material(), &material);
            prepared.into_operation(now())
        });
    let harness = harness(store, identity(fixture.actor.clone()));
    let result = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&expected.group.review),
        )
        .unwrap();
    let MeasureDecisionRecordReceipt::V2(actual) = result else {
        panic!("fresh decision-record submit must retain an actual V2 group")
    };
    assert_eq!(*actual, expected);
    assert_eq!(actual.record_history.records.judicial.groups.len(), 1);
    assert_eq!(actual.record_history.records.administrative.len(), 1);
    assert!(actual.record_history.decisions.is_empty());
    let corrected = &actual.record_history.records.administrative[0].capture;
    let result = &actual.group.measures[0].result;
    assert_eq!(result.values, corrected.review.result.values);
    assert_eq!(result.sources, corrected.review.result.sources);
    assert_eq!(result.record_root, corrected.review.result.record_root);
    assert_eq!(
        result.judicial_origin,
        corrected.review.result.judicial_origin
    );
    assert_eq!(
        result.previous,
        Some(record_support::record_reference(&corrected.records[0]))
    );
    assert_eq!(result.revision.get(), 3);
    assert!(measure_decision_review_v2_bytes(&actual.group.review)
        .unwrap()
        .starts_with(b"MDPR2"));
    assert!(measure_capture_v2_bytes(&actual.group.measures[0])
        .unwrap()
        .starts_with(b"MMCR2"));
    assert!(measure_decision_group_v2_bytes(&actual.group)
        .unwrap()
        .starts_with(b"MDGR2"));
    measure_decision_group_v2_matches(&Hasher, &actual.group, &actual.record_history).unwrap();
    assert_eq!(harness.validator.calls(), 1);
}

#[test]
fn record_workflow_replays_original_v1_after_real_c_and_v2_descendants() {
    let fixture = Fixture::corrected();
    let _descendant = fixture.operation(now());
    let original = fixture.legacy_operation();
    let command = original.group.review.command.clone();
    let expected_command = command.clone();
    let actor = original.group.review.actor.clone();
    let expected_actor = actor.clone();
    let case_id = original.group.review.case_id;
    let returned = original.clone();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(2)
        .withf(move |a, c, cmd, _| {
            *a == expected_actor && *c == case_id && *cmd == expected_command
        })
        .returning(move |_, _, _, _| {
            Ok(MeasureDecisionRecordPreparation::Replay(Box::new(
                MeasureDecisionRecordReceipt::V1(Box::new(returned.clone())),
            )))
        });
    let harness = harness(store, identity(actor));
    let review = harness
        .service
        .prepare("session", case_id, command.clone())
        .unwrap();
    let MeasureDecisionRecordReview::V1(review) = review else {
        panic!("an original V1 replay must keep its captured review family")
    };
    assert_eq!(*review, original.group.review);
    let result = harness
        .service
        .submit(
            "session",
            case_id,
            command,
            MeasureDecisionConfirmation {
                submission_digest: original.group.review.submission_digest,
                review_digest: original.group.review.review_digest,
            },
        )
        .unwrap();
    let MeasureDecisionRecordReceipt::V1(actual) = result else {
        panic!("an original V1 replay must not create a V2 capture")
    };
    assert_eq!(*actual, original);
    assert_eq!(
        measure_decision_group_bytes(&actual.group).unwrap(),
        measure_decision_group_bytes(&original.group).unwrap()
    );
    assert_eq!(harness.validator.calls(), 0);
    assert!(harness.events().is_empty());
}
