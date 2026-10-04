use crate::{
    hearing_derived_deadline_support::{case_id, fixture},
    hearing_derived_deadline_workflow_support::*,
    hearing_result_support::{MockIdentity, Validator},
};
use application::{hearing_derived_deadlines::*, ApplicationError};
use domain::crypto::Sha256Digest;
use std::sync::{atomic::Ordering, Arc};

#[test]
fn prepare_resolves_review_without_commit() {
    let fixture = fixture();
    let expected = fixture.prepare().unwrap();
    let mut store = MockStore::new();
    ready(&mut store, &fixture);
    store.expect_commit().never();
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity(&fixture.actor, 2), validator.clone());
    let result = service
        .prepare("session", case_id(), fixture.command)
        .unwrap();
    let HearingDerivedDeadlineReview::Ready(draft) = result else {
        panic!("new review must not invent a persisted replay")
    };
    assert_eq!(*draft, expected);
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn submit_passes_admitted_result_and_exact_review_to_commit() {
    let fixture = fixture();
    let expected = record(&fixture);
    let returned = expected.clone();
    let draft = fixture.prepare().unwrap();
    let digest = draft.review_digest();
    let actor = fixture.actor.id;
    let mut store = MockStore::new();
    ready(&mut store, &fixture);
    store
        .expect_commit()
        .times(1)
        .return_once(move |id, case, prepared| {
            assert_eq!((id, case), (actor, case_id()));
            assert_eq!(prepared.draft(), &draft);
            assert_eq!(prepared.result().command(), &draft.command().result);
            assert_eq!(
                prepared.result().submission_digest(),
                draft.result().submission_digest
            );
            assert!(prepared.result().formats().is_empty());
            Ok(returned)
        });
    let (service, _) = service(
        store,
        identity(&fixture.actor, 3),
        Arc::new(Validator::default()),
    );
    assert_eq!(
        service
            .submit("session", case_id(), fixture.command, digest)
            .unwrap(),
        expected
    );
}

#[test]
fn replay_returns_captured_record_without_new_admission() {
    let fixture = fixture();
    let expected = record(&fixture);
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| {
            Ok(HearingDerivedDeadlinePreparation::Replay(Box::new(
                returned,
            )))
        });
    store.expect_commit().never();
    let validator = Arc::new(Validator {
        failure: true,
        ..Default::default()
    });
    let (service, _) = service(store, identity(&fixture.actor, 2), validator.clone());
    let reviewed = service
        .prepare("session", case_id(), fixture.command)
        .unwrap();
    let HearingDerivedDeadlineReview::Replay(actual) = reviewed else {
        panic!("historical replay must retain its captured record")
    };
    assert_eq!(*actual, expected);
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn changed_replay_command_or_digest_is_rejected() {
    for changed_command in [false, true] {
        let mut fixture = fixture();
        let returned = record(&fixture);
        let digest = if changed_command {
            fixture.command.result.operation_id =
                application::hearing_results::HearingResultOperationId::new();
            returned.evidence().review_digest
        } else {
            Sha256Digest::from_array([91; 32])
        };
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(HearingDerivedDeadlinePreparation::Replay(Box::new(
                    returned,
                )))
            });
        store.expect_commit().never();
        let (service, _) = service(
            store,
            identity(&fixture.actor, 1),
            Arc::new(Validator::default()),
        );
        assert!(service
            .submit("session", case_id(), fixture.command, digest)
            .is_err());
    }
}

#[test]
fn revoked_session_after_admission_prevents_commit() {
    let fixture = fixture();
    let digest = fixture.prepare().unwrap().review_digest();
    let mut store = MockStore::new();
    ready(&mut store, &fixture);
    store.expect_commit().never();
    let mut auth = MockIdentity::new();
    let mut order = mockall::Sequence::new();
    let actor = fixture.actor.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| Ok(actor));
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    let (service, _) = service(store, auth, Arc::new(Validator::default()));
    assert!(matches!(
        service.submit("session", case_id(), fixture.command, digest),
        Err(ApplicationError::InvalidSession)
    ));
}
