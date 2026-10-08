#[allow(dead_code, unused_imports)]
#[path = "measure_administrative_backend_support/fixture.rs"]
mod administrative_fixture;
#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "precautionary_hearing_backend_support/fixture.rs"]
mod hearing_fixture;
mod measure_decision_record_backend_support;
#[path = "measure_decision_backend_support/fixture.rs"]
mod measure_fixture;
#[allow(unused_imports)]
#[path = "typed_participant_service_support/mod.rs"]
mod typed_participant_service_support;

use measure_decision_record_backend_support::*;

#[test]
fn postgres_record_confirm_retains_actual_corrected_values_and_original_owners() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let reviewed = workflow
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let MeasureDecisionRecordReview::V2(review) = reviewed else {
        panic!("fresh corrected-record decision must use the V2 family")
    };
    assert!(matches!(
        &review.material.predecessors[0],
        OwnedMeasureRecord::Administrative { .. }
    ));
    let stored = workflow
        .submit(
            "session",
            db.case,
            seed.command.clone(),
            confirmation(&review),
        )
        .unwrap();
    let MeasureDecisionRecordReceipt::V2(stored) = stored else {
        panic!("fresh corrected-record decision must persist an actual V2 group")
    };
    assert_eq!(stored.group.review, *review);
    assert_eq!(stored.group.measures.len(), 1);
    let result = &stored.group.measures[0].result;
    let corrected = &seed.corrected.capture.review.result;
    assert_eq!(
        result.previous,
        Some(corrected_reference(&seed.corrected.capture))
    );
    assert_eq!(result.revision.get(), 3);
    assert_eq!(result.values, corrected.values);
    assert_eq!(result.sources, corrected.sources);
    assert_eq!(result.projection, corrected.projection);
    assert_eq!(result.record_root, corrected.record_root);
    assert_eq!(result.judicial_origin, corrected.judicial_origin);
    assert_eq!(stored.record_history.records.judicial.groups.len(), 1);
    assert_eq!(
        stored.record_history.records.judicial.groups[0].capture,
        seed.judicial.group
    );
    assert_eq!(stored.record_history.records.administrative.len(), 1);
    assert_eq!(
        stored.record_history.records.administrative[0].capture,
        seed.corrected.capture
    );
    assert!(stored.record_history.decisions.is_empty());
    assert_eq!(
        stored.origin,
        measure_group_origin_v2(&RingSha256Hasher, &stored.group, &stored.record_history).unwrap()
    );
    assert!(measure_decision_group_v2_bytes(&stored.group)
        .unwrap()
        .starts_with(b"MDGR2"));
    assert!(measure_capture_v2_bytes(&stored.group.measures[0])
        .unwrap()
        .starts_with(b"MMCR2"));
}
