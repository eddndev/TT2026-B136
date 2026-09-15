#[allow(dead_code)]
mod legacy_database_support;

use application::documents::{
    CaseDocumentStore, DocumentAction, DocumentMetadata, DocumentRecord, MetadataQuery,
    MetadataRevision, SealedEvidence, VersionSelection,
};
use application::vault::encrypt_with_ports;
use domain::crypto::{DocumentHasher, DocumentVersion};
use domain::identity::UserId;
use infrastructure::{
    EnvelopeKeyManager, PostgresCaseDocumentStore, RingAesGcmCipher, RingSha256Hasher,
};
use legacy_database_support::{Database, Source, KEK};
use time::OffsetDateTime;

#[test]
fn legacy_reconciliation_preserves_ciphertext_evidence_and_audit_prefix_after_classification() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::with_version(DocumentVersion::new(7).unwrap());
    db.seed_case(source.case_id);
    let document_before = std::fs::read(source.document_path()).unwrap();
    let audit_before = std::fs::read(source.audit_path()).unwrap();
    source.inspect().apply(&db.url).unwrap();
    let actor = UserId::from_uuid(
        db.client
            .query_one(
                "SELECT created_by FROM cases WHERE id=$1",
                &[&source.case_id.as_uuid()],
            )
            .unwrap()
            .get(0),
    );
    let runtime_url = db.restricted_url();
    let store = PostgresCaseDocumentStore::open(&runtime_url).unwrap();
    let at = OffsetDateTime::now_utc();
    assert_eq!(
        store
            .get_metadata(actor, source.case_id, source.id, at)
            .unwrap()
            .metadata_revision
            .get(),
        0
    );
    let mut original = store
        .load(
            actor,
            source.case_id,
            source.id,
            VersionSelection::Only,
            DocumentAction::Verify,
        )
        .unwrap();
    original
        .seal(SealedEvidence {
            signature: vec![1],
            timestamp_token: vec![2],
            signer_certificate_pem: vec![3],
            issuer_certificate_pem: vec![4],
            crl_pem: vec![5],
            tsa_chain_pem: None,
            openssl_version: "fixture".into(),
        })
        .unwrap();
    store
        .seal(actor, source.case_id, original.clone(), at)
        .unwrap();
    let values =
        DocumentMetadata::new(Some("Imported"), Some("Manual"), &["historical".into()]).unwrap();
    store
        .replace_metadata(
            actor,
            source.case_id,
            source.id,
            MetadataRevision::unclassified(),
            values.clone(),
            at,
        )
        .unwrap();
    let version = original.version.next().unwrap();
    let content = b"later encrypted bytes";
    let later = DocumentRecord::pending(
        source.id,
        version,
        "later.txt".into(),
        RingSha256Hasher::new().hash_bytes(content),
        encrypt_with_ports(
            &RingAesGcmCipher::new(),
            &EnvelopeKeyManager::new(),
            &KEK,
            content,
            source.id,
            version,
        )
        .unwrap(),
    )
    .unwrap();
    let overview = store
        .append(actor, source.case_id, original.version, later.clone(), at)
        .unwrap();
    assert_eq!(overview.current_metadata.values, values);
    store
        .replace_metadata(
            actor,
            source.case_id,
            source.id,
            MetadataRevision::new(1),
            DocumentMetadata::empty(),
            at,
        )
        .unwrap();
    let before = store.audit_entries(actor).unwrap();
    let state = db.stored_state();
    let metadata:serde_json::Value=db.client.query_one("SELECT jsonb_agg(to_jsonb(m) ORDER BY metadata_revision) FROM document_metadata_revisions m",&[]).unwrap().get(0);
    source.inspect().check_target(&db.url).unwrap();
    source.inspect().apply(&db.url).unwrap();
    infrastructure::legacy::require_completed_import(source.dir.path(), &runtime_url).unwrap();
    assert_eq!(db.stored_state(), state);
    assert_eq!(store.audit_entries(actor).unwrap(), before);
    assert_eq!(before[..source.entries.len()], source.entries);
    assert_eq!(
        std::fs::read(source.document_path()).unwrap(),
        document_before
    );
    assert_eq!(std::fs::read(source.audit_path()).unwrap(), audit_before);
    let role = reqwest::Url::parse(&runtime_url)
        .unwrap()
        .username()
        .to_string();
    infrastructure::initialize_database(&db.url, &role).unwrap();
    let reopened = PostgresCaseDocumentStore::open(&runtime_url).unwrap();
    assert_eq!(db.client.query_one("SELECT jsonb_agg(to_jsonb(m) ORDER BY metadata_revision) FROM document_metadata_revisions m",&[]).unwrap().get::<_,serde_json::Value>(0),metadata);
    for expected in [original, later] {
        assert_eq!(
            reopened
                .load(
                    actor,
                    source.case_id,
                    source.id,
                    VersionSelection::Exact(expected.version),
                    DocumentAction::Verify
                )
                .unwrap(),
            expected
        );
    }
    let history = reopened
        .metadata_history(
            actor,
            source.case_id,
            source.id,
            MetadataQuery::new(10, None).unwrap(),
            at,
        )
        .unwrap();
    assert_eq!(history.revisions.len(), 2);
    assert_eq!(history.revisions[1].values, values);
}
