use super::*;
use time::Duration;
use uuid::Uuid;

fn history(length: u32) -> PrecautionaryHearingStoredOperation {
    let mut operation = Fixture::schedule().operation(at());
    for revision in 2..=length {
        let mut fixture = Fixture::replace(&operation);
        fixture.command.operation_id = PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(
            1_000 + u128::from(revision),
        ));
        let capture = fixture
            .checked()
            .into_capture(&Hasher, at() + Duration::seconds(1))
            .unwrap();
        operation.history.captures.push(capture.clone());
        operation.capture = capture;
    }
    precautionary_hearing_history_with_measure_history_matches(
        &Hasher,
        &operation.history.captures,
        &operation.history.origin,
        &operation.history.measure_history,
    )
    .unwrap();
    operation
}

#[test]
fn the_hearing_history_budget_includes_the_new_capture_and_accepts_256() {
    let predecessor = history(255);
    let fixture = Fixture::replace(&predecessor);
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(at() + Duration::seconds(100)));
    let harness = harness(store, identity(fixture.actor));
    let result = harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .unwrap();
    assert_eq!(result.history.captures.len(), 256);
    assert_eq!(result.capture.review.result_revision.get(), 256);
    assert_eq!(result.history.captures[..255], predecessor.history.captures);
}

#[test]
fn a_complete_256_capture_predecessor_cannot_be_truncated_to_allow_another_write() {
    let predecessor = history(256);
    let fixture = Fixture::replace(&predecessor);
    let harness = harness(fixture.store(), identity(fixture.actor));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn replay_rejects_an_over_budget_prefix_instead_of_disclosing_only_its_last_capture() {
    let original = history(257);
    let mut fixture = Fixture::schedule();
    fixture.command = original.capture.review.command.clone();
    let harness = harness(fixture.replay_store(original), identity(fixture.actor));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}
