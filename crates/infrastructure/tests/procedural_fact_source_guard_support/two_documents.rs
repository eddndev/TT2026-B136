use super::{case_stage_database_support as stage, procedural_fact_backend_support as backend, *};
use application::{documents::*, identity::Principal, procedural_facts::*, ApplicationError};
use domain::identity::Role;
use infrastructure::RingSha256Hasher;
use std::sync::{Arc, Mutex};

struct CaptureBatch(Arc<Mutex<Vec<usize>>>);
impl DocumentFormatBatchValidator for CaptureBatch {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        self.0.lock().unwrap().push(batch.inputs().len());
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}
fn provenance(record: &DocumentRecord) -> FactProvenance {
    FactProvenance::ExternalReference {
        reference: backend::text("Explicit documentary reference"),
        support: Some(FactEvidence::new(
            DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            record.digest,
            FactLabel::new("page 1").unwrap(),
        )),
    }
}
#[test]
fn integrated_notification_admits_two_direct_documents_without_parent_fanout() {
    let Some(db) = Fixture::new() else { return };
    let parent_document = stage::upload(&db, db.case, "resolution.pdf");
    let practice_document = stage::upload(&db, db.case, "practice.pdf");
    let representation_document = stage::upload(&db, db.case, "representation.pdf");
    let batches = Arc::new(Mutex::new(Vec::new()));
    let workflow = ProceduralFactService::new(
        backend::store(&db),
        Arc::new(stage::TestIdentity(Principal {
            id: db.owner,
            email: "session@example.test".into(),
            role: Role::Owner,
        })),
        stage::processor(),
        Arc::new(CaptureBatch(batches.clone())),
        Arc::new(RingSha256Hasher),
        Arc::new(stage::FixedClock(db.at)),
    );
    let base = backend::values("Declared resolution");
    let parent = backend::persist(
        &workflow,
        db.case,
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            ResolutionId::new(),
            FactChange::record(ResolutionValues::new(ResolutionValuesInput {
                class: base.class().clone(),
                subtype: None,
                issuer: base.issuer().clone(),
                issued_at: base.issued_at(),
                summary: base.summary().clone(),
                provenance: provenance(&parent_document),
            })),
        )),
    );
    batches.lock().unwrap().clear();
    let base =
        backend::notification_values(backend::resolution_ref(&parent), "Declared notification");
    let person = FactPerson::Unlinked {
        label: FactLabel::new("Declared person").unwrap(),
        description: backend::text("Unlinked declaration"),
    };
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: base.resolution(),
        character: base.character().clone(),
        medium: base.medium().clone(),
        context: base.context().clone(),
        outcome: base.outcome().clone(),
        subtype: None,
        practiced_at: base.practiced_at(),
        received_at: None,
        stated_effect: None,
        intended_recipient: base.intended_recipient().clone(),
        actual_receiver: base.actual_receiver().clone(),
        representation: FactRepresentation::Declared {
            represented: person.clone(),
            representative: person,
            scope: backend::text("Declared scope"),
            provenance: Box::new(provenance(&representation_document)),
        },
        summary: base.summary().clone(),
        provenance: provenance(&practice_document),
    })
    .unwrap();
    let detail = backend::persist(
        &workflow,
        db.case,
        ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                NotificationId::new(),
                base.resolution().id,
                FactChange::record(values),
            )
            .unwrap(),
        ),
    );
    assert_eq!(*batches.lock().unwrap(), vec![2, 2]);
    assert_eq!(detail.sources.direct_supports.len(), 2);
    assert!(detail
        .sources
        .direct_supports
        .iter()
        .all(|s| s.reference.id != parent_document.id));
    assert_eq!(
        detail.sources.resolved.resolution.unwrap().reference,
        backend::resolution_ref(&parent)
    );
    assert!(detail.sources.resolved.participants.is_empty());
    assert!(detail.sources.resolved.hearing_results.is_empty());
    assert_eq!(
        workflow
            .get(
                "session",
                db.case,
                detail.snapshot.target(),
                Some(detail.snapshot.metadata().revision)
            )
            .unwrap(),
        detail
    );
}
