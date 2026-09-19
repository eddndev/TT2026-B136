#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod procedural_fact_service_support;
mod procedural_resource_support;
use application::{procedural_resources::*, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use procedural_resource_support::*;
use std::sync::Arc;

#[test]
fn registration_captures_exact_resolution_support_and_declared_appellant() {
    let (identity, actor) = case_support::identity(Role::Litigator, 2);
    let fixture = Fixture::new(&actor);
    let mut store = MockStore::new();
    let material = fixture.material.clone();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(ResourcePreparation::Ready(Box::new(material))));
    let admission = Arc::new(Validator::default());
    let service = service(store, identity, admission.clone());
    let draft = service
        .prepare("session", fixture.case_id, fixture.command.clone())
        .unwrap();
    assert_eq!(draft.values, fixture.values);
    assert_eq!(draft.recorded_by.id, actor.id);
    assert_eq!(draft.recorded_by.email, actor.email);
    assert_eq!(
        draft.sources.resolution.snapshot.reference,
        fixture.values.resolution()
    );
    assert_eq!(draft.sources.supports[0].reference, fixture.record_ref());
    assert_eq!(
        draft.sources.appellants[0].overview.display_name,
        "Historical appellant"
    );
    assert_eq!(
        draft.values.appellants()[0].name().as_str(),
        "Declared appellant"
    );
    assert_eq!(draft.result_revision, ResourceRevision::initial());
    assert_eq!(admission.calls(), 1);
}

#[test]
fn client_and_paralegal_cannot_prepare_and_store_denials_are_preserved() {
    for role in [Role::Client, Role::Paralegal] {
        let (identity, actor) = case_support::identity(role, 1);
        let fixture = Fixture::new(&actor);
        let service = service(MockStore::new(), identity, Arc::new(Validator::default()));
        assert!(matches!(
            service.prepare("session", fixture.case_id, fixture.command),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    let (identity, actor) = case_support::identity(Role::Litigator, 1);
    let fixture = Fixture::new(&actor);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(|_, _, _, _| Err(ApplicationError::PermissionDenied));
    let service = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn changed_email_after_admission_cannot_return_a_draft() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let mut identity = case_support::MockIdentity::new();
    let first = actor.clone();
    let mut second = actor;
    second.email = "changed@example.test".into();
    let mut calls = 0;
    identity.expect_authenticate().times(2).returning(move |_| {
        calls += 1;
        Ok(if calls == 1 {
            first.clone()
        } else {
            second.clone()
        })
    });
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(ResourcePreparation::Ready(Box::new(fixture.material))));
    let service = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn wrong_resolution_case_or_revision_and_document_digest_fail_before_admission() {
    for field in 0..3 {
        let (identity, actor) = case_support::identity(Role::Owner, 1);
        let mut fixture = Fixture::new(&actor);
        match field {
            0 => fixture.material.case_id = CaseId::new(),
            1 => {
                fixture
                    .material
                    .resolution
                    .as_mut()
                    .unwrap()
                    .snapshot
                    .metadata
                    .revision = domain::procedural_facts::FactRevision::new(2).unwrap()
            }
            _ => {
                fixture.material.records[0].digest =
                    domain::crypto::Sha256Digest::from_array([9; 32])
            }
        }
        let mut store = MockStore::new();
        store.expect_prepare().return_once(move |_, _, _, _| {
            Ok(ResourcePreparation::Ready(Box::new(fixture.material)))
        });
        let admission = Arc::new(Validator::default());
        let service = service(store, identity, admission.clone());
        assert!(service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert_eq!(admission.calls(), 0);
    }
}

#[test]
fn submit_and_replay_preserve_the_exact_receipt_without_another_commit() {
    let (identity, actor) = case_support::identity(Role::Owner, 2);
    let fixture = Fixture::new(&actor);
    let draft = prepared_draft(&fixture, &actor);
    let expected = draft.submission_digest;
    let mut store = MockStore::new();
    let material = fixture.material.clone();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(ResourcePreparation::Ready(Box::new(material))));
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_detail(case_support::instant()));
    let workflow = service(store, identity, Arc::new(Validator::default()));
    let result = workflow
        .submit(
            "session",
            fixture.case_id,
            fixture.command.clone(),
            expected,
        )
        .unwrap();
    assert_eq!(result.receipt.operation_id, fixture.command.operation_id);
    assert_eq!(result.values, fixture.values);
    let original = result.clone();
    let (identity, _) = identity_for(actor, 2);
    let mut replay = MockStore::new();
    replay
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(ResourcePreparation::Replay(Box::new(result))));
    let service = service(replay, identity, Arc::new(Validator::default()));
    assert_eq!(
        service
            .submit("session", fixture.case_id, fixture.command, expected)
            .unwrap(),
        original
    );
}
