mod case_support;

use application::cases::{CaseAccess, CaseService, CaseWorkflow};
use application::ApplicationError;
use case_support::{identity, record, MockCases, MockIdentity};
use domain::cases::CaseId;
use domain::identity::{Role, UserId};
use std::sync::Arc;

#[test]
fn creator_is_taken_from_authenticated_identity_and_metadata_is_normalized() {
    for role in [Role::Owner, Role::Litigator] {
        let (auth, actor) = identity(role, 1);
        let mut repository = MockCases::new();
        repository
            .expect_insert()
            .times(1)
            .withf(move |case| {
                case.created_by == actor.id
                    && case.title == "Defense file"
                    && case.reference == "NUC-123"
            })
            .returning(|_| Ok(()));
        let service = CaseService::new(Arc::new(repository), Arc::new(auth));
        let case = service
            .create("session", " Defense file ", " NUC-123 ")
            .unwrap();
        assert_eq!(case.created_by, actor.id);
    }
}

#[test]
fn non_creators_and_non_owners_cannot_mutate_even_when_assigned() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let (auth, _) = identity(role, if role == Role::Litigator { 2 } else { 3 });
        let service = CaseService::new(Arc::new(MockCases::new()), Arc::new(auth));
        if role != Role::Litigator {
            assert!(matches!(
                service.create("session", "File", "123"),
                Err(ApplicationError::PermissionDenied)
            ));
        }
        assert!(matches!(
            service.assign("session", CaseId::new(), UserId::new()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            service.remove("session", CaseId::new(), UserId::new()),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn list_uses_current_identity_scope_for_every_role() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let (auth, actor) = identity(role, 1);
        let expected = if role == Role::Owner {
            CaseAccess::All
        } else {
            CaseAccess::Assigned(actor.id)
        };
        let case = record(actor.id);
        let returned = case.clone();
        let mut repository = MockCases::new();
        repository
            .expect_list()
            .times(1)
            .withf(move |access, limit, offset| {
                *access == expected && *limit == 10 && *offset == 20
            })
            .returning(move |_, _, _| Ok(vec![returned.clone()]));
        let service = CaseService::new(Arc::new(repository), Arc::new(auth));
        assert_eq!(service.list("session", 10, 20).unwrap(), vec![case]);
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
        .expect_find()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(move |candidate, access| {
            *candidate == id && *access == CaseAccess::Assigned(actor.id)
        })
        .return_once(move |_, _| Ok(Some(case)));
    repository
        .expect_find()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(move |candidate, access| {
            *candidate == id && *access == CaseAccess::Assigned(actor.id)
        })
        .return_once(|_, _| Ok(None));
    let service = CaseService::new(Arc::new(repository), Arc::new(auth));
    assert_eq!(service.get("session", id).unwrap().id, id);
    assert!(matches!(
        service.get("session", id),
        Err(ApplicationError::CaseNotFound)
    ));
}

#[test]
fn unknown_and_foreign_cases_have_the_same_error() {
    let (auth, actor) = identity(Role::Paralegal, 2);
    let mut repository = MockCases::new();
    repository
        .expect_find()
        .times(2)
        .withf(move |_, access| *access == CaseAccess::Assigned(actor.id))
        .returning(|_, _| Ok(None));
    let service = CaseService::new(Arc::new(repository), Arc::new(auth));
    for id in [CaseId::new(), CaseId::new()] {
        assert!(matches!(
            service.get("session", id),
            Err(ApplicationError::CaseNotFound)
        ));
    }
}

#[test]
fn every_operation_rejects_invalid_sessions_before_repository_access() {
    let mut auth = MockIdentity::new();
    auth.expect_authenticate()
        .times(5)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let service = CaseService::new(Arc::new(MockCases::new()), Arc::new(auth));
    let id = CaseId::new();
    let user = UserId::new();
    assert!(matches!(
        service.create("revoked", "File", "123"),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.list("revoked", 10, 0),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.get("revoked", id),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.assign("revoked", id, user),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.remove("revoked", id, user),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn owner_assigns_and_removes_exact_resource_and_target() {
    let (auth, _) = identity(Role::Owner, 2);
    let id = CaseId::new();
    let user = UserId::new();
    let mut repository = MockCases::new();
    repository
        .expect_add_member()
        .times(1)
        .withf(move |c, u| *c == id && *u == user)
        .returning(|_, _| Ok(()));
    repository
        .expect_remove_member()
        .times(1)
        .withf(move |c, u| *c == id && *u == user)
        .returning(|_, _| Ok(()));
    let service = CaseService::new(Arc::new(repository), Arc::new(auth));
    service.assign("session", id, user).unwrap();
    service.remove("session", id, user).unwrap();
}

#[test]
fn metadata_and_page_limits_are_validated_before_persistence() {
    let (auth, _) = identity(Role::Owner, 4);
    let service = CaseService::new(Arc::new(MockCases::new()), Arc::new(auth));
    assert!(matches!(
        service.create("session", " ", "123"),
        Err(ApplicationError::Domain(_))
    ));
    assert!(matches!(
        service.create("session", "File", ""),
        Err(ApplicationError::Domain(_))
    ));
    for limit in [0, 101] {
        assert!(matches!(
            service.list("session", limit, 0),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
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
        .returning(|_, _| Ok(()));
    let service = CaseService::new(Arc::new(repository), Arc::new(auth));
    let case_id = CaseId::new();
    service.assign("session", case_id, id).unwrap();
    assert!(matches!(
        service.assign("session", case_id, id),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn repository_failures_are_not_reported_as_success() {
    let (auth, _) = identity(Role::Owner, 1);
    let mut repository = MockCases::new();
    repository
        .expect_insert()
        .times(1)
        .returning(|_| Err(ApplicationError::Port("unavailable".into())));
    let service = CaseService::new(Arc::new(repository), Arc::new(auth));
    assert!(matches!(
        service.create("session", "File", "123"),
        Err(ApplicationError::Port(_))
    ));
}
