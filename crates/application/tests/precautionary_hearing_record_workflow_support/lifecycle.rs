use super::*;

#[test]
fn prepare_and_submit_review_of_a_real_corrected_record_preserves_exact_mixed_proof() {
    let fixture = Fixture::schedule();
    let expected_review = fixture.review();
    let prepared = harness(fixture.store(), identity(fixture.actor.clone()));
    let review = prepared
        .service
        .prepare("session", fixture.case_id, fixture.command.clone())
        .unwrap();
    assert_eq!(review, expected_review);
    assert_eq!(prepared.validator.calls(), 1);
    assert_eq!(prepared.events(), vec!["unwrap", "open", "hash"]);

    let expected = fixture.operation(now());
    let mut store = fixture.store();
    let actor = fixture.actor.clone();
    let material = fixture.material.clone();
    let checked = expected_review.clone();
    store
        .expect_commit()
        .times(1)
        .return_once(move |actual, _, prepared| {
            assert_eq!(actual, &actor);
            assert_eq!(prepared.actor(), &actor);
            assert_eq!(prepared.material(), &material);
            assert_eq!(prepared.review(), &checked);
            prepared.into_operation(now())
        });
    let submitted = harness(store, identity(fixture.actor));
    let operation = submitted
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(operation, expected);
    assert_eq!(
        operation
            .history
            .record_history
            .records
            .administrative
            .len(),
        1
    );
    assert_eq!(
        operation
            .history
            .record_history
            .records
            .judicial
            .groups
            .len(),
        1
    );
    assert!(operation.history.record_history.decisions.is_empty());
    assert_eq!(submitted.validator.calls(), 1);
}
