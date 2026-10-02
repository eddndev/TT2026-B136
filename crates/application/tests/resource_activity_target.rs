#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
#[allow(dead_code)]
mod hearing_support;
use deadline_support::evaluation::inputs::facts as procedural_fact_service_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::{identity::Principal, resource_activities::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    hearings::HearingId,
    identity::{Role, UserId},
};
use resource_activity_support::*;

fn query() -> ResourceActivityTargetQuery {
    ResourceActivityTargetQuery::new(2, None, Some(ResourceActivityStatus::Linked)).unwrap()
}
fn target(row: &ResourceActivityDetail) -> ResourceActivityTargetId {
    match row.selection.target {
        ResourceActivityTarget::Hearing { id, .. } => ResourceActivityTargetId::Hearing(id),
        ResourceActivityTarget::Deadline { id, .. } => ResourceActivityTargetId::Deadline(id),
    }
}
fn page(rows: Vec<ResourceActivityView>) -> ResourceActivityTargetPage {
    ResourceActivityTargetPage {
        checked_at: case_support::instant(),
        associations: rows,
        has_more: false,
        next_after_id: None,
    }
}
fn identity(actor: &Principal, calls: usize) -> case_support::MockIdentity {
    let mut identity = case_support::MockIdentity::new();
    let principal = actor.clone();
    identity
        .expect_authenticate()
        .times(calls)
        .returning(move |_| Ok(principal.clone()));
    identity
}
fn invalid(result: Result<ResourceActivityTargetPage, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::StoredInconsistent(_)
        ))
    ));
}

#[test]
fn target_query_rejects_unbounded_pages_and_preserves_typed_filters() {
    for limit in [0, 101, u32::MAX] {
        assert!(ResourceActivityTargetQuery::new(limit, None, None).is_err());
    }
    let id = ResourceActivityId::new();
    let query =
        ResourceActivityTargetQuery::new(100, Some(id), Some(ResourceActivityStatus::Unlinked))
            .unwrap();
    assert_eq!(query.limit(), 100);
    assert_eq!(query.after_id(), Some(id));
    assert_eq!(query.status(), Some(ResourceActivityStatus::Unlinked));
}

#[test]
fn staff_receive_verified_exact_associations_with_one_observation_and_empty_pages() {
    let (_, owner) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&owner);
    let row = fixture.committed(&owner);
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        for empty in [false, true] {
            let mut actor = owner.clone();
            actor.role = role;
            let expected = page(if empty {
                vec![]
            } else {
                vec![hearing_view(row.clone())]
            });
            let returned = expected.clone();
            let requested_target = target(&row);
            let case = row.case_id;
            let actor_id = actor.id;
            let mut store = MockStore::new();
            store
                .expect_list_for_target()
                .times(1)
                .withf(move |a, c, t, q, at| {
                    *a == actor_id
                        && *c == case
                        && *t == requested_target
                        && *q == query()
                        && *at == case_support::instant()
                })
                .return_once(move |_, _, _, _, _| Ok(returned));
            let actual = service(store, identity(&actor, 2))
                .list_for_target("session", case, requested_target, query())
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn target_page_rejects_wrong_scope_capture_status_order_and_pagination() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let row = fixture.committed(&actor);
    for fault in 0..11 {
        let mut returned = page(vec![hearing_view(row.clone())]);
        let mut wanted = target(&row);
        let mut case = row.case_id;
        let mut q = query();
        match fault {
            0 => case = CaseId::new(),
            1 => wanted = ResourceActivityTargetId::Hearing(HearingId::new()),
            2 => {
                returned.associations[0].association.receipt.capture_digest =
                    Sha256Digest::from_array([8; 32])
            }
            3 => {
                q = ResourceActivityTargetQuery::new(
                    2,
                    None,
                    Some(ResourceActivityStatus::Unlinked),
                )
                .unwrap()
            }
            4 => returned.associations.push(returned.associations[0].clone()),
            5 => returned.next_after_id = Some(row.id),
            6 => returned.has_more = true,
            7 => {
                returned.has_more = true;
                returned.next_after_id = Some(row.id);
            }
            8 => q = ResourceActivityTargetQuery::new(2, Some(row.id), None).unwrap(),
            9 => returned.associations[0].checked_at += time::Duration::nanoseconds(1),
            _ => returned.checked_at -= time::Duration::nanoseconds(1),
        }
        let mut store = MockStore::new();
        store
            .expect_list_for_target()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(returned));
        invalid(service(store, identity(&actor, 1)).list_for_target("session", case, wanted, q));
    }
}

#[test]
fn empty_page_still_rejects_invalid_observation_and_continuation() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    for fault in 0..4 {
        let mut returned = page(vec![]);
        match fault {
            0 => returned.checked_at -= time::Duration::seconds(1),
            1 => returned.checked_at += time::Duration::seconds(1),
            2 => returned.has_more = true,
            _ => returned.next_after_id = Some(ResourceActivityId::new()),
        }
        let mut store = MockStore::new();
        store
            .expect_list_for_target()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(returned));
        invalid(service(store, identity(&actor, 1)).list_for_target(
            "session",
            CaseId::new(),
            ResourceActivityTargetId::Hearing(HearingId::new()),
            query(),
        ));
    }
}

#[test]
fn client_is_denied_before_storage_and_complete_principal_is_reauthenticated() {
    let (client, _) = case_support::identity(Role::Client, 1);
    assert!(matches!(
        service(MockStore::new(), client).list_for_target(
            "session",
            CaseId::new(),
            ResourceActivityTargetId::Hearing(HearingId::new()),
            query()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    let (_, actor) = case_support::identity(Role::Owner, 0);
    for fault in 0..4 {
        let mut changed = actor.clone();
        match fault {
            0 => changed.id = UserId::new(),
            1 => changed.email = "changed@example.test".into(),
            _ => changed.role = Role::Litigator,
        }
        let mut identity = case_support::MockIdentity::new();
        let first = actor.clone();
        let mut sequence = mockall::Sequence::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(first));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                if fault == 3 {
                    Err(ApplicationError::InvalidSession)
                } else {
                    Ok(changed)
                }
            });
        let mut store = MockStore::new();
        store
            .expect_list_for_target()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(page(vec![])));
        assert!(matches!(
            service(store, identity).list_for_target(
                "session",
                CaseId::new(),
                ResourceActivityTargetId::Hearing(HearingId::new()),
                query()
            ),
            Err(ApplicationError::InvalidSession)
        ));
    }
}
