#![allow(dead_code)]
use crate::{case_support, hearing_support, procedural_resource_support as resources};
use application::{
    identity::Principal, procedural_resources::*, resource_activities::*, ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Store {}
    impl ResourceActivityStore for Store {
        fn list(&self, actor:UserId, case:CaseId, resource:ResourceId, query:ResourceActivityQuery, at:OffsetDateTime)->Result<ResourceActivityPage,ApplicationError>;
        fn get(&self, actor:UserId, case:CaseId, resource:ResourceId, id:ResourceActivityId, revision:Option<ResourceActivityRevision>, at:OffsetDateTime)->Result<ResourceActivityView,ApplicationError>;
        fn history(&self, actor:UserId, case:CaseId, resource:ResourceId, id:ResourceActivityId, query:ResourceActivityHistoryQuery, at:OffsetDateTime)->Result<ResourceActivityHistoryPage,ApplicationError>;
        fn prepare(&self, actor:UserId, case:CaseId, resource:ResourceId, command:&ResourceActivityCommand)->Result<ResourceActivityPreparation,ApplicationError>;
        fn commit(&self, actor:UserId, case:CaseId, resource:ResourceId, prepared:PreparedResourceActivityChange)->Result<ResourceActivityDetail,ApplicationError>;
    }
}
pub fn service(store: MockStore, identity: case_support::MockIdentity) -> ResourceActivityService {
    ResourceActivityService::new(
        Arc::new(store),
        Arc::new(identity),
        hearing_support::hasher(),
        Arc::new(case_support::CountingClock::default()),
    )
}
pub struct Fixture {
    pub case_id: CaseId,
    pub resource_id: ResourceId,
    pub command: ResourceActivityCommand,
    pub material: ResourceActivityMaterial,
}
impl Fixture {
    pub fn new(actor: &Principal) -> Self {
        let mut fixture = resources::Fixture::new(actor);
        fixture.case_id = crate::deadline_support::evaluation::inputs::case_id();
        fixture.material.case_id = fixture.case_id;
        let facts = crate::procedural_fact_service_support::values();
        let fact_command = application::procedural_facts::ProceduralFactCommand::Resolution(
            application::procedural_facts::ResolutionCommand::new(
                application::procedural_facts::FactOperationId::new(),
                fixture.values.resolution().id,
                application::procedural_facts::FactChange::record(facts.clone()),
            ),
        );
        let source = crate::procedural_fact_service_support::detail(
            actor.id,
            fixture.case_id,
            &fact_command,
            facts,
            crate::procedural_fact_service_support::empty(),
        );
        let application::procedural_facts::ProceduralFactSnapshot::Resolution(snapshot) =
            source.snapshot
        else {
            unreachable!()
        };
        fixture.material.resolution.as_mut().unwrap().snapshot = *snapshot;
        for person in &mut fixture.material.appellants {
            let application::typed_participants::ParticipantRevisionSnapshot::Manual(snapshot) =
                &mut person.revision
            else {
                unreachable!()
            };
            snapshot.case_id = fixture.case_id;
        }
        let original = resources::commit(
            actor,
            fixture.case_id,
            fixture.command.clone(),
            fixture.material.clone(),
        );
        let mut act_material = resources::retained(original.clone());
        act_material.records = fixture.material.records;
        let act_command = ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: original.id,
            change: ResourceChange::RecordAct {
                expected_revision: original.revision,
                act_id: ResourceActId::new(),
                values: resources::act_values(
                    fixture.values.resolution_evidence().clone(),
                    "Original declared act",
                ),
            },
        };
        let act = resources::commit(actor, fixture.case_id, act_command, act_material);
        let mut changed = resources::retained(act.clone());
        changed.act_base = Some(act.clone());
        let old_act = act.act.as_ref().unwrap();
        let correction = ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: original.id,
            change: ResourceChange::CorrectAct {
                expected_revision: act.revision,
                act_id: old_act.id,
                expected_act_revision: old_act.revision,
                values: resources::act_values(
                    fixture.values.resolution_evidence().clone(),
                    "Corrected declared act",
                ),
                reason: resources::text("Clarify declaration"),
            },
        };
        let head = resources::commit(actor, fixture.case_id, correction, changed);
        let hearing_command = hearing_support::command();
        let context = hearing_support::context(fixture.case_id, actor.id);
        let hearing = hearing_support::detail(
            fixture.case_id,
            actor.id,
            &hearing_command,
            hearing_support::values(),
            &context,
        );
        let selection = ResourceActivitySelection {
            resource: ResourceCaptureRef {
                id: original.id,
                revision: original.revision,
                capture_digest: original.receipt.capture_digest,
            },
            act: Some(ResourceActCaptureRef {
                id: old_act.id,
                revision: old_act.revision,
                resource_revision: act.revision,
                capture_digest: act.receipt.capture_digest,
            }),
            target: ResourceActivityTarget::Hearing {
                id: hearing.snapshot.id,
                revision: hearing.snapshot.revision,
                submission_digest: hearing.snapshot.receipt.submission_digest,
            },
        };
        Self {
            case_id: fixture.case_id,
            resource_id: original.id,
            command: ResourceActivityCommand {
                operation_id: ResourceActivityOperationId::new(),
                association_id: ResourceActivityId::new(),
                expected_resource_revision: head.revision,
                change: ResourceActivityChange::Link { selection },
            },
            material: ResourceActivityMaterial {
                case_id: fixture.case_id,
                base: None,
                administration: head.recorded_administration.clone(),
                resource_head: head,
                sources: ResourceActivitySources {
                    resource: original,
                    act: Some(act),
                    target: ResourceActivityTargetDetail::Hearing(Box::new(hearing)),
                },
            },
        }
    }
    pub fn draft(&self, actor: &Principal) -> ResourceActivityDraft {
        let (identity, _) = resources::identity_for(actor.clone(), 2);
        let mut store = MockStore::new();
        let material = self.material.clone();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(ResourceActivityPreparation::Ready(Box::new(material)))
            });
        service(store, identity)
            .prepare(
                "session",
                self.case_id,
                self.resource_id,
                self.command.clone(),
            )
            .unwrap()
    }
    pub fn use_deadline(&mut self) {
        let (command, material) = crate::deadline_support::fixture();
        let target = crate::deadline_support::detail(
            &crate::deadline_support::prepare(command, material).unwrap(),
        );
        assert_eq!(target.case_id, self.case_id);
        let ResourceActivityChange::Link { selection } = &mut self.command.change else {
            unreachable!()
        };
        selection.target = ResourceActivityTarget::Deadline {
            id: target.id,
            revision: target.revision,
            capture_digest: target.receipt.capture_digest,
        };
        self.material.sources.target = ResourceActivityTargetDetail::Deadline(Box::new(target));
    }
    pub fn committed(&self, actor: &Principal) -> ResourceActivityDetail {
        let expected = self.draft(actor).submission_digest;
        let (identity, _) = resources::identity_for(actor.clone(), 2);
        let mut store = MockStore::new();
        let material = self.material.clone();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(ResourceActivityPreparation::Ready(Box::new(material)))
            });
        store
            .expect_commit()
            .times(1)
            .return_once(|_, _, _, prepared| prepared.into_detail(case_support::instant()));
        service(store, identity)
            .submit(
                "session",
                self.case_id,
                self.resource_id,
                self.command.clone(),
                expected,
            )
            .unwrap()
    }
    pub fn archive_resource(&mut self, actor: &Principal) {
        let head = self.material.resource_head.clone();
        let command = ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: head.id,
            change: ResourceChange::Archive {
                expected_revision: head.revision,
                reason: resources::text("Organizational archive"),
            },
        };
        self.material.resource_head =
            resources::commit(actor, self.case_id, command, resources::retained(head));
        self.command.expected_resource_revision = self.material.resource_head.revision;
    }
    pub fn unlink(&mut self, base: ResourceActivityDetail) {
        self.command.operation_id = ResourceActivityOperationId::new();
        self.command.change = ResourceActivityChange::Unlink {
            expected_revision: base.revision,
            reason: resources::text("Organizational unlink"),
        };
        self.material.base = Some(base);
    }
}
pub fn hearing_view(association: ResourceActivityDetail) -> ResourceActivityView {
    let ResourceActivityTargetDetail::Hearing(target) = association.sources.target.clone() else {
        unreachable!()
    };
    ResourceActivityView {
        association,
        checked_at: case_support::instant(),
        current_target: ResourceActivityCurrentTarget::Hearing(target),
    }
}
