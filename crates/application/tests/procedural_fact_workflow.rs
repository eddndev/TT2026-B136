#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod procedural_fact_service_support;
#[allow(dead_code)]
#[path = "procedural_fact_service_support/store.rs"]
mod store;
use application::{procedural_facts::*, ApplicationError};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use procedural_fact_service_support::*;
use std::sync::{atomic::Ordering, Arc};
use store::*;
#[test]
fn prepare_without_documents_needs_no_profile_stage_or_clock() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let cmd = command();
    let prep = preparation(case);
    let expected = prep.observed_administration.clone();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |id, c, _, limits| {
            assert_eq!((id, c), (actor.id, case));
            assert_eq!(limits.max_documents(), 2);
            Ok(prep)
        });
    let validator = Arc::new(Validator::default());
    let (service, clock) = service(store, identity, validator.clone());
    let draft = service.prepare("session", case, cmd.clone()).unwrap();
    assert_eq!(draft.command, cmd);
    assert_eq!(draft.actor, actor.id);
    assert_eq!(draft.sources, empty());
    assert_eq!(draft.observed_administration, expected);
    assert_eq!(draft.result_revision.get(), 1);
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    assert_eq!(clock.calls(), 0);
}
#[test]
fn submit_reprepares_and_checks_committed_operation() {
    let (identity, actor) = identity(Role::Litigator, 2);
    let case = CaseId::new();
    let cmd = command();
    let prep = preparation(case);
    let expected = detail(actor.id, case, &cmd, values(), empty());
    let output = expected.clone();
    let digest = output.snapshot.metadata().receipt.submission_digest;
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |id, c, prepared| {
            assert_eq!((id, c), (actor.id, case));
            assert_eq!(prepared.submission_digest(), digest);
            assert_eq!(prepared.sources(), &empty());
            Ok(output)
        });
    let (service, clock) = service(store, identity, Arc::new(Validator::default()));
    assert_eq!(
        service.submit("session", case, cmd, digest).unwrap(),
        expected
    );
    assert_eq!(clock.calls(), 0);
}
#[test]
fn direct_support_is_admitted_once_and_projected() {
    let (identity, _) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let record = record();
    let cmd = record_command(with_support(&record));
    let expected = record.clone();
    let mut prep = preparation(case);
    prep.records = vec![record];
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    let draft = service.prepare("session", case, cmd).unwrap();
    assert_eq!(validator.calls.load(Ordering::SeqCst), 1);
    assert_eq!(draft.sources.direct_supports[0].reference.id, expected.id);
    assert_eq!(draft.sources.direct_supports[0].digest, expected.digest);
}
#[test]
fn unauthorized_roles_never_touch_store_or_clock() {
    for role in [Role::Paralegal, Role::Client] {
        let (identity, _) = identity(role, 1);
        let (service, clock) = service(MockStore::new(), identity, Arc::new(Validator::default()));
        assert!(matches!(
            service.prepare("session", CaseId::new(), command()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(clock.calls(), 0);
    }
}
#[test]
fn mismatching_preview_never_commits() {
    let (identity, _) = identity(Role::Owner, 1);
    let case = CaseId::new();
    let prep = preparation(case);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.submit("session", case, command(), digest(77)),
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::SubmissionMismatch
        ))
    ));
}
#[test]
fn session_revocation_and_actor_change_after_preparation_never_commit() {
    for other_actor in [false, true] {
        let mut identity = MockIdentity::new();
        let actor = application::identity::Principal {
            id: UserId::new(),
            email: "actor@example.com".into(),
            role: Role::Owner,
        };
        let original = actor.clone();
        let mut seq = mockall::Sequence::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(move |_| Ok(original));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(move |_| {
                if other_actor {
                    Ok(application::identity::Principal {
                        id: UserId::new(),
                        ..actor
                    })
                } else {
                    Err(ApplicationError::InvalidSession)
                }
            });
        let case = CaseId::new();
        let prep = preparation(case);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(matches!(
            service.prepare("session", case, command()),
            Err(ApplicationError::InvalidSession)
        ));
    }
}
#[test]
fn foreign_preparation_and_unselected_documents_are_rejected_before_admission() {
    for extra in [false, true] {
        let (identity, _) = identity(Role::Owner, 1);
        let case = CaseId::new();
        let mut prep = preparation(if extra { case } else { CaseId::new() });
        if extra {
            prep.records.push(record());
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .return_once(move |_, _, _, _| Ok(prep));
        let validator = Arc::new(Validator::default());
        let (service, _) = service(store, identity, validator.clone());
        assert!(matches!(
            service.prepare("session", case, command()),
            Err(ApplicationError::ProceduralFact(
                ProceduralFactError::StoredInconsistent(_)
            ))
        ));
        assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn format_failure_maps_to_fact_error_without_reauthentication() {
    let (identity, _) = identity(Role::Owner, 1);
    let case = CaseId::new();
    let record = record();
    let cmd = record_command(with_support(&record));
    let mut prep = preparation(case);
    prep.records = vec![record];
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator {
        failure: true,
        ..Default::default()
    });
    let (service, _) = service(store, identity, validator);
    assert!(matches!(
        service.prepare("session", case, cmd),
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::SupportFormatRejected
        ))
    ));
}
#[test]
fn withdrawal_preserves_every_source_without_readmission() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let base = detail(actor.id, case, &command(), values(), empty());
    let FactTarget::Resolution(id) = base.snapshot.target() else {
        unreachable!()
    };
    let cmd = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        id,
        FactChange::withdraw(FactRevision::initial(), text("Withdraw declaration")),
    ));
    let mut prep = preparation(case);
    prep.base = Some(base.clone());
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    let draft = service.prepare("session", case, cmd).unwrap();
    assert_eq!(draft.values, base.snapshot.values());
    assert_eq!(draft.sources, base.sources);
    assert_eq!(draft.result_revision.get(), 2);
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}
