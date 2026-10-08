use super::*;

#[test]
fn real_administrative_capture_supports_a_durable_review_hearing_with_exact_mixed_proof() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let operation = persist(&db, seed.actor.clone(), seed.command.clone());
    assert_eq!(operation.capture.review.command, seed.command);
    assert_eq!(operation.capture.recorded_at, db.at);
    assert_eq!(operation.history.captures, vec![operation.capture.clone()]);
    let evidence = &operation.history.record_history;
    assert_eq!(evidence.records.judicial.groups.len(), 1);
    assert_eq!(
        evidence.records.judicial.groups[0].capture,
        seed.judicial.group
    );
    assert_eq!(evidence.records.administrative.len(), 1);
    assert_eq!(
        evidence.records.administrative[0].capture,
        seed.corrected.capture
    );
    assert_eq!(
        evidence.records.administrative[0].origin,
        seed.corrected.origin
    );
    assert!(evidence.decisions.is_empty());
    assert_eq!(
        operation.capture.review.resolved_values.review_targets(),
        &[crate::administrative_fixture::corrected_reference(
            &seed.corrected.capture
        )]
    );
    precautionary_hearing_history_with_decision_history_matches(
        &RingSha256Hasher,
        &operation.history.captures,
        &operation.history.origin,
        evidence,
    )
    .unwrap();
    reopened(&db, &seed.actor, &operation);
}
