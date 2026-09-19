#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod deadline_support;
#[allow(dead_code)]
mod hearing_support;
#[allow(dead_code)]
mod procedural_fact_service_support;
mod procedural_resource_support;
mod resource_activity_support;

use application::{identity::Principal, resource_activities::*, ApplicationError};
use domain::{
    clock::{Clock, OffsetDateTime},
    identity::Role,
};
use resource_activity_support::{Fixture, MockStore};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use time::Duration;

struct ReadClock {
    start: OffsetDateTime,
    end: OffsetDateTime,
    started: AtomicBool,
}
impl Clock for ReadClock {
    fn now(&self) -> OffsetDateTime {
        if self.started.swap(true, Ordering::SeqCst) {
            self.end
        } else {
            self.start
        }
    }
}
fn start() -> OffsetDateTime {
    case_support::instant() + Duration::seconds(10)
}
fn finish() -> OffsetDateTime {
    start() + Duration::seconds(10)
}
fn workflow(store: MockStore, actor: &Principal) -> ResourceActivityService {
    let principal = actor.clone();
    let mut identity = case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .times(1..=2)
        .returning(move |_| Ok(principal.clone()));
    ResourceActivityService::new(
        Arc::new(store),
        Arc::new(identity),
        hearing_support::hasher(),
        Arc::new(ReadClock {
            start: start(),
            end: finish(),
            started: AtomicBool::new(false),
        }),
    )
}
fn late_view(
    mut row: ResourceActivityDetail,
    actor: &Principal,
    checked: OffsetDateTime,
) -> ResourceActivityView {
    row.recorded_at = checked;
    row.receipt.capture_digest =
        hearing_support::hasher().hash_bytes(&resource_activity_capture_bytes(&row));
    resource_activity_receipt_matches(hearing_support::hasher().as_ref(), &row).unwrap();
    let ResourceActivityTargetDetail::Hearing(original) = &row.sources.target else {
        unreachable!()
    };
    let mut current = hearing_support::detail(
        row.case_id,
        actor.id,
        &hearing_support::replacement(original),
        original.snapshot.values.clone(),
        &hearing_support::context(row.case_id, actor.id),
    );
    current.snapshot.recorded_at = checked;
    application::hearings::hearing_receipt_matches(hearing_support::hasher().as_ref(), &current)
        .unwrap();
    ResourceActivityView {
        association: row,
        checked_at: checked,
        current_target: ResourceActivityCurrentTarget::Hearing(Box::new(current)),
    }
}
fn two_views(
    fixture: &mut Fixture,
    actor: &Principal,
    first: OffsetDateTime,
    second: OffsetDateTime,
) -> Vec<ResourceActivityView> {
    let one = late_view(fixture.committed(actor), actor, first);
    fixture.command.association_id = ResourceActivityId::new();
    fixture.command.operation_id = ResourceActivityOperationId::new();
    let two = late_view(fixture.committed(actor), actor, second);
    let mut rows = vec![one, two];
    rows.sort_by_key(|view| view.association.id.as_uuid());
    rows
}
fn invalid(result: Result<ResourceActivityPage, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::StoredInconsistent(_)
        ))
    ));
}

#[test]
fn exact_read_accepts_capture_committed_after_request_started() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let original = fixture.committed(&actor);
    let checked = start() + Duration::seconds(5) + Duration::nanoseconds(1);
    let expected = late_view(original.clone(), &actor, checked);
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .withf(|_, _, _, _, _, at| *at == start())
        .return_once(move |_, _, _, _, _, _| Ok(returned));
    let actual = workflow(store, &actor)
        .get(
            "session",
            fixture.case_id,
            fixture.resource_id,
            original.id,
            Some(original.revision),
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.association.sources, original.sources);
}

#[test]
fn page_accepts_one_common_observation_between_request_and_return() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let mut fixture = Fixture::new(&actor);
    let checked = start() + Duration::seconds(5);
    let expected = two_views(&mut fixture, &actor, checked, checked);
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .withf(|_, _, _, _, at| *at == start())
        .return_once(move |_, _, _, _, _| {
            Ok(ResourceActivityPage {
                associations: returned,
                has_more: false,
                next_after_id: None,
            })
        });
    let actual = workflow(store, &actor)
        .list(
            "session",
            fixture.case_id,
            fixture.resource_id,
            ResourceActivityQuery::new(2, None, None, None).unwrap(),
        )
        .unwrap();
    assert_eq!(actual.associations, expected);
}

#[test]
fn current_reads_reject_observations_outside_the_request_window() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    for checked in [
        start() - Duration::nanoseconds(1),
        finish() + Duration::nanoseconds(1),
    ] {
        let mut value = resource_activity_support::hearing_view(fixture.committed(&actor));
        value.checked_at = checked;
        let returned = value.clone();
        let mut store = MockStore::new();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _, _| Ok(returned));
        assert!(matches!(
            workflow(store, &actor).get(
                "session",
                fixture.case_id,
                fixture.resource_id,
                value.association.id,
                None
            ),
            Err(ApplicationError::ResourceActivity(
                ResourceActivityError::StoredInconsistent(_)
            ))
        ));
        let mut store = MockStore::new();
        store
            .expect_list()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(ResourceActivityPage {
                    associations: vec![value],
                    has_more: false,
                    next_after_id: None,
                })
            });
        invalid(workflow(store, &actor).list(
            "session",
            fixture.case_id,
            fixture.resource_id,
            ResourceActivityQuery::new(2, None, None, None).unwrap(),
        ));
    }
}

#[test]
fn page_rejects_different_observation_instants_even_inside_the_window() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let mut fixture = Fixture::new(&actor);
    let checked = start() + Duration::seconds(5);
    let rows = two_views(
        &mut fixture,
        &actor,
        checked,
        checked + Duration::nanoseconds(1),
    );
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _, _, _| {
            Ok(ResourceActivityPage {
                associations: rows,
                has_more: false,
                next_after_id: None,
            })
        });
    invalid(workflow(store, &actor).list(
        "session",
        fixture.case_id,
        fixture.resource_id,
        ResourceActivityQuery::new(2, None, None, None).unwrap(),
    ));
}
