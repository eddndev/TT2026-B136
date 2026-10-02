#![allow(dead_code)]
use application::{
    deadline_tracking::*, deadlines::*, identity::Principal, resource_activities::*,
    resource_deadlines::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use mockall::mock;
use std::sync::Arc;
mock! {
    pub Store {}
    impl ResourceDeadlineStore for Store {
        fn prepare(&self, actor:UserId, case:CaseId, resource:ResourceId, command:&ResourceDeadlineCommand)->Result<ResourceDeadlinePreparation,ApplicationError>;
        fn commit(&self, actor:UserId, case:CaseId, resource:ResourceId, prepared:PreparedResourceDeadline)->Result<ResourceDeadlineResult,ApplicationError>;
    }
}
pub fn service(
    store: MockStore,
    identity: crate::case_support::MockIdentity,
) -> ResourceDeadlineService {
    ResourceDeadlineService::new(
        Arc::new(store),
        Arc::new(identity),
        crate::hearing_support::hasher(),
        Arc::new(crate::case_support::CountingClock::default()),
    )
}
#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub command: ResourceDeadlineCommand,
    pub material: ResourceDeadlineMaterial,
}
impl Fixture {
    pub fn new(role: Role) -> Self {
        let mut actor = Principal {
            id: crate::deadline_support::owner(),
            email: "owner@example.com".into(),
            role: Role::Owner,
        };
        let fixture = crate::resource_activity_support::Fixture::new(&actor);
        actor.role = role;
        let ResourceActivityChange::Link { selection } = fixture.command.change else {
            unreachable!()
        };
        let (command, deadline) = crate::deadline_support::fixture();
        Self {
            actor,
            command: ResourceDeadlineCommand {
                association_id: fixture.command.association_id,
                expected_resource_revision: fixture.command.expected_resource_revision,
                resource: selection.resource,
                act: selection.act,
                deadline: DeadlineHumanCommand::new(
                    command,
                    Some(TrackingPolicies {
                        profile: TrackingPolicy::Follow,
                        source: TrackingPolicy::Follow,
                        calendar: TrackingPolicy::Undetermined,
                    }),
                )
                .unwrap(),
            },
            material: ResourceDeadlineMaterial {
                case_id: fixture.case_id,
                administration: fixture.material.administration,
                resource_head: fixture.material.resource_head,
                resource: fixture.material.sources.resource,
                act: fixture.material.sources.act,
                deadline,
            },
        }
    }
    pub fn case(&self) -> CaseId {
        self.material.case_id
    }
    pub fn resource(&self) -> ResourceId {
        self.command.resource.id
    }
    pub fn store(&self) -> MockStore {
        let mut store = MockStore::new();
        let material = self.material.clone();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(ResourceDeadlinePreparation::Ready(Box::new(material)))
            });
        store
    }
    pub fn service(&self, calls: usize) -> ResourceDeadlineService {
        let (identity, _) =
            crate::procedural_resource_support::identity_for(self.actor.clone(), calls);
        service(self.store(), identity)
    }
    pub fn preview(&self) -> ResourceDeadlineDraft {
        self.service(2)
            .prepare(
                "session",
                self.case(),
                self.resource(),
                self.command.clone(),
            )
            .unwrap()
    }
}
