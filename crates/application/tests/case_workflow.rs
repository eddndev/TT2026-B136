mod case_support;

use application::cases::{CaseRecord, CaseWorkflow};
use application::ApplicationError;
use case_support::{identity, instant, record, service, MockCases, MockIdentity};
use domain::cases::CaseId;
use domain::identity::{Role, UserId};

#[test]
fn creator_is_taken_from_authenticated_identity_and_metadata_is_normalized() {
    for role in [Role::Owner, Role::Litigator] {
        let (auth, actor) = identity(role, 1);
        let mut repository = MockCases::new();
        repository
            .expect_create_basic()
            .times(1)
            .withf(move |who, id, metadata, at| {
                *who == actor.id
                    && id.as_uuid().get_version_num() == 4
                    && metadata.title() == "Defense file"
                    && metadata.reference() == "NUC-123"
                    && *at == instant()
            })
            .returning(|actor, id, metadata, _| {
                Ok(CaseRecord {
                    id,
                    title: metadata.title().into(),
                    reference: metadata.reference().into(),
                    created_by: actor,
                })
            });
        let (workflow, clock) = service(repository, auth);
        let case = workflow
            .create("session", " Defense file ", " NUC-123 ")
            .unwrap();
        assert_eq!(case.created_by, actor.id);
        assert_eq!(case.title, "Defense file");
        assert_eq!(case.reference, "NUC-123");
        assert_eq!(clock.calls(), 1);
    }
}

#[test]
fn non_creators_and_non_owners_cannot_mutate_even_when_assigned() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let (auth, _) = identity(role, if role == Role::Litigator { 2 } else { 3 });
        let (workflow, clock) = service(MockCases::new(), auth);
        if role != Role::Litigator {
            assert!(matches!(
                workflow.create("session", "File", "123"),
                Err(ApplicationError::PermissionDenied)
            ));
        }
        assert!(matches!(
            workflow.assign("session", CaseId::new(), UserId::new()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            workflow.remove("session", CaseId::new(), UserId::new()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn list_uses_current_identity_for_audited_scope_for_every_role() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let (auth, actor) = identity(role, 1);
        let case = record(actor.id);
        let returned = case.clone();
        let mut repository = MockCases::new();
        repository
            .expect_list_basic()
            .times(1)
            .withf(move |who, limit, offset, at| {
                *who == actor.id && *limit == 10 && *offset == 20 && *at == instant()
            })
            .returning(move |_, _, _, _| Ok(vec![returned.clone()]));
        let (workflow, clock) = service(repository, auth);
        assert_eq!(workflow.list("session", 10, 20).unwrap(), vec![case]);
        assert_eq!(clock.calls(), 1);
    }
}

#[test]
fn revoked_membership_is_observed_on_next_request_with_same_session() {
    let (auth, actor) = identity(Role::Client, 2);
    let case = record(UserId::new());
    let id = case.id;
    let mut repository = MockCases::new();
    let mut sequence = mockall::Sequence::new();
    repository
        .expect_get_basic()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(move |who, candidate, at| *who == actor.id && *candidate == id && *at == instant())
        .return_once(move |_, _, _| Ok(case));
    repository
        .expect_get_basic()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(move |who, candidate, at| *who == actor.id && *candidate == id && *at == instant())
        .return_once(|_, _, _| Err(ApplicationError::CaseNotFound));
    let (workflow, clock) = service(repository, auth);
    assert_eq!(workflow.get("session", id).unwrap().id, id);
    assert!(matches!(
        workflow.get("session", id),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(clock.calls(), 2);
}

#[test]
fn unknown_and_foreign_cases_have_the_same_error() {
    let (auth, actor) = identity(Role::Paralegal, 2);
    let mut repository = MockCases::new();
    repository
        .expect_get_basic()
        .times(2)
        .withf(move |who, _, at| *who == actor.id && *at == instant())
        .returning(|_, _, _| Err(ApplicationError::CaseNotFound));
    let (workflow, clock) = service(repository, auth);
    for id in [CaseId::new(), CaseId::new()] {
        assert!(matches!(
            workflow.get("session", id),
            Err(ApplicationError::CaseNotFound)
        ));
    }
    assert_eq!(clock.calls(), 2);
}

#[test]
fn every_operation_rejects_invalid_sessions_before_repository_access() {
    let mut auth = MockIdentity::new();
    auth.expect_authenticate()
        .times(5)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let (workflow, clock) = service(MockCases::new(), auth);
    let id = CaseId::new();
    let user = UserId::new();
    for error in [
        workflow.create("revoked", "File", "123").unwrap_err(),
        workflow.list("revoked", 10, 0).unwrap_err(),
        workflow.get("revoked", id).unwrap_err(),
        workflow.assign("revoked", id, user).unwrap_err(),
        workflow.remove("revoked", id, user).unwrap_err(),
    ] {
        assert!(matches!(error, ApplicationError::InvalidSession));
    }
    assert_eq!(clock.calls(), 0);
}

#[test]
fn owner_assigns_and_removes_exact_resource_and_target() {
    let (auth, actor) = identity(Role::Owner, 2);
    let id = CaseId::new();
    let user = UserId::new();
    let mut repository = MockCases::new();
    repository
        .expect_add_member()
        .times(1)
        .withf(move |c, u, a, at| *c == id && *u == user && *a == actor.id && *at == instant())
        .returning(|_, _, _, _| Ok(()));
    repository
        .expect_remove_member()
        .times(1)
        .withf(move |c, u, a, at| *c == id && *u == user && *a == actor.id && *at == instant())
        .returning(|_, _, _, _| Ok(()));
    let (workflow, clock) = service(repository, auth);
    workflow.assign("session", id, user).unwrap();
    workflow.remove("session", id, user).unwrap();
    assert_eq!(clock.calls(), 2);
}

#[test]
fn metadata_and_page_limits_are_validated_before_persistence() {
    let (workflow, clock) = service(MockCases::new(), identity(Role::Owner, 4).0);
    assert!(matches!(
        workflow.create("session", " ", "123"),
        Err(ApplicationError::Domain(_))
    ));
    assert!(matches!(
        workflow.create("session", "File", ""),
        Err(ApplicationError::Domain(_))
    ));
    for limit in [0, 101] {
        assert!(matches!(
            workflow.list("session", limit, 0),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    assert_eq!(clock.calls(), 0);
}

#[test]
fn current_role_is_reloaded_before_each_membership_mutation() {
    let mut auth = MockIdentity::new();
    let id = UserId::new();
    let mut sequence = mockall::Sequence::new();
    for role in [Role::Owner, Role::Paralegal] {
        auth.expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                Ok(application::identity::Principal {
                    id,
                    email: "owner@example.com".into(),
                    role,
                })
            });
    }
    let mut repository = MockCases::new();
    repository
        .expect_add_member()
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    let (workflow, clock) = service(repository, auth);
    let case_id = CaseId::new();
    workflow.assign("session", case_id, id).unwrap();
    assert!(matches!(
        workflow.assign("session", case_id, id),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 1);
}

#[test]
fn repository_failures_are_not_reported_as_success() {
    let (auth, _) = identity(Role::Owner, 1);
    let mut repository = MockCases::new();
    repository
        .expect_create_basic()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::Port("unavailable".into())));
    let (workflow, clock) = service(repository, auth);
    assert!(matches!(
        workflow.create("session", "File", "123"),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(clock.calls(), 1);
}
