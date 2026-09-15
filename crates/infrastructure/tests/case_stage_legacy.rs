mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod legacy_database_support;

use application::case_stages::*;
use application::documents::{DocumentProcessor, DocumentRepository};
use application::identity::Principal;
use case_stage_database_support::*;
use domain::identity::Role;
use infrastructure::{
    EnvelopeKeyManager, FileDocumentRepository, RingAesGcmCipher, RingSha256Hasher,
};
use legacy_database_support::{Source, KEK};
use std::sync::Arc;
use zeroize::Zeroizing;

#[test]
fn imported_legacy_receipt_reconciles_after_adoption_without_rewriting_stage_or_audit_history() {
    let Some(mut db) = Fixture::new() else { return };
    let mut source = Source::with_version(domain::crypto::DocumentVersion::new(7).unwrap());
    source.case_id = db.case;
    std::fs::write(&source.mapping,serde_json::to_vec(&serde_json::json!({"documents":[{"document_id":source.id.to_string(),"case_id":db.case.to_string()}]})).unwrap()).unwrap();
    source.inspect().apply(&db.admin_url).unwrap();
    complete(&db);
    let record = FileDocumentRepository::new(source.dir.path().join("documents"))
        .unwrap()
        .find(source.id)
        .unwrap()
        .unwrap();
    let mut ports = crypto::processor_ports();
    ports.hasher = Box::new(RingSha256Hasher);
    ports.cipher = Box::new(RingAesGcmCipher::new());
    ports.keys = Box::new(EnvelopeKeyManager::new());
    let processor = DocumentProcessor::new(
        ports,
        crypto::evidence_material(),
        Zeroizing::new(KEK.to_vec()),
    )
    .unwrap();
    let workflow = CaseStageService::new(
        store(&db),
        Arc::new(TestIdentity(Principal {
            id: db.owner,
            email: "owner@example.test".into(),
            role: Role::Owner,
        })),
        Arc::new(processor),
        Arc::new(FormatCheck(None)),
        Arc::new(FixedClock(db.at)),
    );
    let detail = workflow
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Investigation),
        )
        .unwrap();
    let before = snapshot(&mut db);
    source.inspect().check_target(&db.admin_url).unwrap();
    source.inspect().apply(&db.admin_url).unwrap();
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(store(&db).get(db.owner, db.case, db.at).unwrap(), detail);
    let prefix: Vec<String> = db
        .admin
        .query(
            "SELECT timestamp FROM audit_events WHERE sequence<2 ORDER BY sequence",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(
        prefix,
        source
            .entries
            .iter()
            .map(|entry| entry.event.timestamp_rfc3339().unwrap())
            .collect::<Vec<_>>()
    );
}
