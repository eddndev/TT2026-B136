#![allow(dead_code)]
use application::{
    identity::Principal, resource_activities::*, resource_hearings::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    hearings::*,
    identity::{Role, UserId},
    resource_hearings::*,
};
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Store {}
    impl ResourceHearingStore for Store {
        fn prepare(&self, actor: UserId, case: CaseId, resource: ResourceId, command: &ResourceHearingCommand) -> Result<ResourceHearingPreparation, ApplicationError>;
        fn commit(&self, actor: UserId, case: CaseId, resource: ResourceId, prepared: PreparedResourceHearing) -> Result<ResourceHearingCreation, ApplicationError>;
    }
}
pub fn service(
    store: MockStore,
    identity: crate::case_support::MockIdentity,
) -> ResourceHearingService {
    ResourceHearingService::new(
        Arc::new(store),
        Arc::new(identity),
        crate::hearing_support::hasher(),
        Arc::new(crate::case_support::CountingClock::default()),
    )
}
#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub command: ResourceHearingCommand,
    pub material: ResourceHearingMaterial,
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
        let support = &fixture.material.sources.resource.sources.supports[0];
        let values = ResourceHearingValues::new(ResourceHearingValuesInput {
            kind: ResourceHearingKind::WrittenRevocation,
            scheduled_at: crate::hearing_support::values().scheduled_at(),
            modality: HearingModality::InPerson,
            venue: HearingVenue::new("Declared court room").unwrap(),
            note: None,
            participants: vec![],
            scheduling_basis: ResourceHearingSchedulingBasis::new(
                HearingNote::new("Court declared the hearing for written revocation").unwrap(),
                HearingSupportRef::new(support.reference, support.digest),
            ),
        })
        .unwrap();
        Self {
            actor,
            command: ResourceHearingCommand {
                operation_id: ResourceHearingOperationId::new(),
                hearing_id: ResourceHearingId::new(),
                association_id: fixture.command.association_id,
                expected_resource_revision: fixture.command.expected_resource_revision,
                resource: selection.resource,
                act: selection.act,
                values,
            },
            material: ResourceHearingMaterial {
                case_id: fixture.case_id,
                administration: fixture.material.administration,
                resource_head: fixture.material.resource_head,
                resource: fixture.material.sources.resource,
                act: fixture.material.sources.act,
                participants: vec![],
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
                Ok(ResourceHearingPreparation::Ready(Box::new(material)))
            });
        store
    }
    pub fn prepare(&self) -> Result<ResourceHearingDraft, ApplicationError> {
        let (identity, _) = crate::procedural_resource_support::identity_for(self.actor.clone(), 2);
        service(self.store(), identity).prepare(
            "session",
            self.case(),
            self.resource(),
            self.command.clone(),
        )
    }
    pub fn prepare_rejected(&self) -> Result<ResourceHearingDraft, ApplicationError> {
        let (identity, _) = crate::procedural_resource_support::identity_for(self.actor.clone(), 1);
        service(self.store(), identity).prepare(
            "session",
            self.case(),
            self.resource(),
            self.command.clone(),
        )
    }
    pub fn change_values(&mut self, edit: impl FnOnce(&mut ResourceHearingValuesInput)) {
        let values = &self.command.values;
        let mut input = ResourceHearingValuesInput {
            kind: values.kind(),
            scheduled_at: values.scheduled_at(),
            modality: values.modality(),
            venue: values.venue().clone(),
            note: values.note().cloned(),
            participants: values.participants().to_vec(),
            scheduling_basis: values.scheduling_basis().clone(),
        };
        edit(&mut input);
        self.command.values = ResourceHearingValues::new(input).unwrap();
    }
}
