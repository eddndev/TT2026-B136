#![allow(dead_code)]
use crate::{
    case_support::{CountingClock, MockIdentity},
    procedural_fact_service_support as facts,
};
use application::{
    case_stages::CurrentCaseStage,
    documents::{
        DocumentFormatBatch, DocumentFormatBatchValidator, StageDocumentFormat,
        StageSupportReadLimits,
    },
    identity::Principal,
    procedural_facts::*,
    procedural_resources::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
};
use mockall::mock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

mock! {
    pub Store {}
    impl ProceduralResourceStore for Store {
        fn list(&self, actor:UserId, case_id:CaseId, query:ResourceQuery, at:OffsetDateTime)->Result<ResourcePage,ApplicationError>;
        fn get(&self, actor:UserId, case_id:CaseId, id:ResourceId, revision:Option<ResourceRevision>, at:OffsetDateTime)->Result<ResourceDetail,ApplicationError>;
        fn history(&self, actor:UserId, case_id:CaseId, id:ResourceId, query:ResourceHistoryQuery, at:OffsetDateTime)->Result<ResourceHistoryPage,ApplicationError>;
        fn prepare(&self, actor:UserId, case_id:CaseId, command:&ResourceCommand, limits:&StageSupportReadLimits)->Result<ResourcePreparation,ApplicationError>;
        fn commit(&self, actor:UserId, case_id:CaseId, prepared:PreparedResourceChange)->Result<ResourceDetail,ApplicationError>;
    }
}
#[derive(Default)]
pub struct Validator(AtomicUsize);
impl Validator {
    pub fn calls(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}
impl DocumentFormatBatchValidator for Validator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}
pub fn service(
    store: MockStore,
    identity: MockIdentity,
    validator: Arc<Validator>,
) -> ProceduralResourceService {
    ProceduralResourceService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(crate::crypto::processor()),
        validator,
        facts::hasher(),
        Arc::new(CountingClock::default()),
    )
}
pub fn identity_for(principal: Principal, calls: usize) -> (MockIdentity, Principal) {
    let mut identity = MockIdentity::new();
    let returned = principal.clone();
    identity
        .expect_authenticate()
        .times(calls)
        .returning(move |_| Ok(returned.clone()));
    (identity, principal)
}
pub fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub struct Fixture {
    pub case_id: CaseId,
    pub material: ResourceMaterial,
    pub values: ResourceValues,
    pub command: ResourceCommand,
}
impl Fixture {
    pub fn new(actor: &Principal) -> Self {
        let case_id = CaseId::new();
        let source_command = facts::command();
        let source = facts::detail(
            actor.id,
            case_id,
            &source_command,
            facts::values(),
            facts::empty(),
        );
        let ProceduralFactSnapshot::Resolution(snapshot) = source.snapshot else {
            unreachable!()
        };
        let reference = FactResolutionRef {
            id: snapshot.root.id(),
            revision: snapshot.metadata.revision,
        };
        let record = crate::crypto::processor()
            .prepare_version(
                DocumentId::new(),
                DocumentVersion::initial(),
                "resource-evidence.pdf",
                b"%PDF-exact resource evidence",
            )
            .unwrap();
        let evidence = FactEvidence::new(
            DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            record.digest,
            label("Page 1"),
        );
        let participant_values = domain::participants::ParticipantValues::new(
            "Historical appellant",
            "Historical role",
            None,
            None,
            domain::participants::DirectoryStatus::Archived,
        )
        .unwrap();
        let participant = application::participants::ParticipantSnapshot {
            case_id,
            id: domain::participants::ParticipantId::new(),
            revision: domain::participants::ParticipantRevision::new(2).unwrap(),
            values_digest: application::participants::participant_digest(
                facts::hasher().as_ref(),
                &participant_values,
            ),
            values: participant_values,
            changed_at: crate::case_support::instant(),
            changed_by: application::participants::ParticipantActorSnapshot {
                id: actor.id,
                email: actor.email.clone(),
            },
        };
        let selected = FactParticipantRef {
            id: participant.id,
            revision: participant.revision,
        };
        let values = ResourceValues::new(ResourceValuesInput {
            kind: ResourceKind::Revocation,
            mode: FactDeclaration::Known(ResourceMode::Written),
            title: label("Declared resource"),
            resolution: reference,
            resolution_evidence: evidence,
            resolution_reference: FactDeclaration::Unknown(text("Not stated")),
            issuing_authority: FactDeclaration::Unknown(text("Not stated")),
            receiving_authority: None,
            resolution_at: DeclaredProceduralTime::unknown(),
            notification_at: None,
            challenged_part: text("Declared challenged part"),
            grounds: text("Declared grounds"),
            appellants: vec![ResourceAppellant::new(
                label("Declared appellant"),
                FactDeclaration::Known(label("Declared role")),
                Some(selected),
            )],
        })
        .unwrap();
        let command = ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: ResourceId::new(),
            change: ResourceChange::Register {
                values: values.clone(),
            },
        };
        let material = ResourceMaterial {
            case_id,
            base: None,
            act_base: None,
            administration: facts::preparation(case_id).observed_administration,
            stage: CurrentCaseStage::Unregistered,
            resolution: Some(Box::new(ResolutionSourceMaterial {
                snapshot: *snapshot,
                hearing: None,
                admitted_support: None,
            })),
            appellants: vec![participant.into()],
            records: vec![record],
        };
        Self {
            case_id,
            material,
            values,
            command,
        }
    }
    pub fn record_ref(&self) -> DocumentVersionRef {
        let record = &self.material.records[0];
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        }
    }
}
pub fn prepared_draft(fixture: &Fixture, actor: &Principal) -> ResourceDraft {
    let (identity, _) = identity_for(actor.clone(), 2);
    let mut store = MockStore::new();
    let material = fixture.material.clone();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(ResourcePreparation::Ready(Box::new(material))));
    service(store, identity, Arc::new(Validator::default()))
        .prepare("session", fixture.case_id, fixture.command.clone())
        .unwrap()
}
pub fn commit(
    actor: &Principal,
    case_id: CaseId,
    command: ResourceCommand,
    material: ResourceMaterial,
) -> ResourceDetail {
    let (identity, _) = identity_for(actor.clone(), 4);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(2)
        .returning(move |_, _, _, _| Ok(ResourcePreparation::Ready(Box::new(material.clone()))));
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_detail(crate::case_support::instant()));
    let service = service(store, identity, Arc::new(Validator::default()));
    let draft = service
        .prepare("session", case_id, command.clone())
        .unwrap();
    service
        .submit("session", case_id, command, draft.submission_digest)
        .unwrap()
}
pub fn retained(base: ResourceDetail) -> ResourceMaterial {
    ResourceMaterial {
        case_id: base.case_id,
        administration: base.recorded_administration.clone(),
        stage: base.recorded_stage.clone(),
        base: Some(base),
        act_base: None,
        resolution: None,
        appellants: vec![],
        records: vec![],
    }
}
pub fn act_values(evidence: FactEvidence, statement: &str) -> ResourceActValues {
    ResourceActValues::new(ResourceActValuesInput {
        kind: ResourceActKind::Interposition,
        mode: FactDeclaration::Known(ResourceMode::Written),
        occurred_at: DeclaredProceduralTime::unknown(),
        authority: FactDeclaration::Unknown(text("Not stated")),
        statement: text(statement),
        evidence: vec![evidence],
    })
    .unwrap()
}
