use crate::{
    hearing_derived_deadline_support::{case_id, fixture, Fixture},
    hearing_derived_deadline_workflow_support::*,
    hearing_result_support::{MockIdentity, Validator},
};
use application::{
    documents::DocumentRecord, hearing_derived_deadlines::*, hearing_results::*, ApplicationError,
};
use domain::{
    crypto::{DocumentVersionRef, Sha256Digest},
    identity::{Role, UserId},
};
use std::sync::{atomic::Ordering, Arc};

#[test]
fn unauthorized_roles_never_reach_preparation_or_commit() {
    for role in [Role::Paralegal, Role::Client] {
        let mut fixture = fixture();
        fixture.actor.role = role;
        let mut store = MockStore::new();
        store.expect_prepare().never();
        store.expect_commit().never();
        let (service, _) = service(
            store,
            identity(&fixture.actor, 1),
            Arc::new(Validator::default()),
        );
        assert!(matches!(
            service.prepare("session", case_id(), fixture.command),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn fresh_database_actor_must_match_authenticated_principal() {
    let fixture = fixture();
    let mut input = inputs(&fixture);
    input.actor.role = Role::Litigator;
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| {
            Ok(HearingDerivedDeadlinePreparation::Ready(Box::new(input)))
        });
    store.expect_commit().never();
    let (service, _) = service(
        store,
        identity(&fixture.actor, 1),
        Arc::new(Validator::default()),
    );
    assert!(service
        .prepare("session", case_id(), fixture.command)
        .is_err());
}

#[test]
fn replay_preserves_original_authority_after_current_identity_changes() {
    let mut fixture = fixture();
    let expected = record(&fixture);
    let returned = expected.clone();
    fixture.actor.role = Role::Litigator;
    fixture.actor.email = "updated@example.com".into();
    let digest = expected.evidence().review_digest;
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
        identity(&fixture.actor, 2),
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
fn replay_for_another_actor_is_rejected_before_returning_history() {
    let mut fixture = fixture();
    let returned = record(&fixture);
    fixture.actor.id = UserId::new();
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
        .prepare("session", case_id(), fixture.command)
        .is_err());
}

#[test]
fn wrong_review_never_reaches_commit() {
    let fixture = fixture();
    let mut store = MockStore::new();
    ready(&mut store, &fixture);
    store.expect_commit().never();
    let (service, _) = service(
        store,
        identity(&fixture.actor, 1),
        Arc::new(Validator::default()),
    );
    assert!(service
        .submit(
            "session",
            case_id(),
            fixture.command,
            Sha256Digest::from_array([3; 32])
        )
        .is_err());
}

#[test]
fn mismatched_committed_record_is_not_returned_as_success() {
    let fixture = fixture();
    let digest = fixture.prepare().unwrap().review_digest();
    let other = record(&crate::hearing_derived_deadline_support::fixture());
    let mut store = MockStore::new();
    ready(&mut store, &fixture);
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(other));
    let (service, _) = service(
        store,
        identity(&fixture.actor, 2),
        Arc::new(Validator::default()),
    );
    assert!(service
        .submit("session", case_id(), fixture.command, digest)
        .is_err());
}

#[test]
fn support_admission_precedes_refreshed_authentication() {
    let mut fixture = fixture();
    let record = support(&mut fixture);
    let mut store = MockStore::new();
    ready(&mut store, &fixture);
    store.expect_commit().never();
    let validator = Arc::new(Validator::default());
    let mut auth = MockIdentity::new();
    let mut order = mockall::Sequence::new();
    let actor = fixture.actor.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| Ok(actor));
    let actor = fixture.actor.clone();
    let checked = validator.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| {
            assert_eq!(checked.calls.load(Ordering::SeqCst), 1);
            Ok(actor)
        });
    let (service, _) = service(store, auth, validator);
    let HearingDerivedDeadlineReview::Ready(draft) = service
        .prepare("session", case_id(), fixture.command)
        .unwrap()
    else {
        panic!("expected new review")
    };
    let admitted = draft.result().support.as_ref().unwrap();
    assert_eq!(admitted.reference.id, record.id);
    assert_eq!(admitted.digest, record.digest);
    assert_eq!(admitted.name, record.name);
}

#[test]
fn rejected_support_never_reaches_commit() {
    let mut fixture = fixture();
    support(&mut fixture);
    let mut store = MockStore::new();
    ready(&mut store, &fixture);
    store.expect_commit().never();
    let validator = Arc::new(Validator {
        failure: true,
        ..Default::default()
    });
    let (service, _) = service(store, identity(&fixture.actor, 1), validator.clone());
    assert!(service
        .submit(
            "session",
            case_id(),
            fixture.command,
            Sha256Digest::from_array([3; 32])
        )
        .is_err());
    assert_eq!(validator.calls.load(Ordering::SeqCst), 1);
}

fn support(fixture: &mut Fixture) -> DocumentRecord {
    let processor = crate::crypto::processor();
    let record = processor
        .seal(
            &processor
                .prepare("minutes.pdf", b"Declared oral result")
                .unwrap(),
        )
        .unwrap();
    let HearingResultChange::Record { values, .. } = &mut fixture.command.result.change else {
        unreachable!()
    };
    let mut input = crate::hearing_result_support::values_input(values);
    input.provenance = HearingResultProvenance::new(
        HearingResultProvenanceKind::OralReference,
        Some(HearingResultReference::new("Declared oral communication").unwrap()),
        Some(HearingResultSupportRef::new(
            DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            record.digest,
        )),
    )
    .unwrap();
    *values = HearingResultValues::new(input).unwrap();
    fixture.result_preparation.records = vec![record.clone()];
    record
}
