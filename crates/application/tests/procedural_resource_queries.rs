#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod procedural_fact_service_support;
mod procedural_resource_support;
use application::{procedural_resources::*, ApplicationError};
use domain::identity::Role;
use procedural_resource_support::*;
use std::sync::Arc;

#[test]
fn technical_page_bounds_and_positive_revisions_are_explicit() {
    for limit in [0, 101] {
        assert!(ResourceQuery::new(limit, None, None, None).is_err());
    }
    assert!(ResourceQuery::new(
        100,
        None,
        Some(ResourceKind::Appeal),
        Some(ResourceStatus::Archived)
    )
    .is_ok());
    assert!(ResourceHistoryQuery::new(21, None).is_err());
    assert!(ResourceHistoryQuery::new(20, Some(0)).is_err());
}

#[test]
fn every_staff_role_reads_after_reauthentication_but_client_never_reaches_store() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let (identity, actor) =
            case_support::identity(role, if role == Role::Client { 1 } else { 2 });
        let fixture = Fixture::new(&actor);
        let mut store = MockStore::new();
        if role != Role::Client {
            store.expect_list().times(1).return_once(|_, _, _, _| {
                Ok(ResourcePage {
                    resources: vec![],
                    has_more: false,
                    next_after_id: None,
                })
            });
        }
        let service = service(store, identity, Arc::new(Validator::default()));
        let result = service.list(
            "session",
            fixture.case_id,
            ResourceQuery::new(10, None, None, None).unwrap(),
        );
        if role == Role::Client {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        } else {
            assert!(result.unwrap().resources.is_empty());
        }
    }
}

#[test]
fn incoherent_empty_page_is_not_disclosed() {
    let (identity, actor) = case_support::identity(Role::Owner, 1);
    let fixture = Fixture::new(&actor);
    let mut store = MockStore::new();
    store.expect_list().return_once(|_, _, _, _| {
        Ok(ResourcePage {
            resources: vec![],
            has_more: true,
            next_after_id: Some(ResourceId::new()),
        })
    });
    let service = service(store, identity, Arc::new(Validator::default()));
    assert!(service
        .list(
            "session",
            fixture.case_id,
            ResourceQuery::new(10, None, None, None).unwrap()
        )
        .is_err());
}

#[test]
fn exact_detail_never_substitutes_the_current_revision() {
    let (identity, actor) = case_support::identity(Role::Paralegal, 1);
    let fixture = Fixture::new(&actor);
    let mut owner = actor.clone();
    owner.role = Role::Owner;
    let detail = commit(&owner, fixture.case_id, fixture.command, fixture.material);
    let id = detail.id;
    let mut store = MockStore::new();
    store
        .expect_get()
        .return_once(move |_, _, _, _, _| Ok(detail));
    let service = service(store, identity, Arc::new(Validator::default()));
    assert!(service
        .get(
            "session",
            fixture.case_id,
            id,
            Some(ResourceRevision::new(2).unwrap())
        )
        .is_err());
}
