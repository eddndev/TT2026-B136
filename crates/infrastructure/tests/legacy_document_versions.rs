#[allow(dead_code)]
mod legacy_database_support;

use application::documents::{
    CaseDocumentStore, DocumentAction, DocumentRecord, VersionQuery, VersionSelection,
};
use application::vault::encrypt_with_ports;
use domain::audit::AuditLog;
use domain::crypto::{DocumentHasher, DocumentVersion};
use domain::identity::UserId;
use infrastructure::{
    EnvelopeKeyManager, PostgresAuditLog, PostgresCaseDocumentStore, RingAesGcmCipher,
    RingSha256Hasher,
};
use legacy_database_support::{Database, Source, KEK};
use time::OffsetDateTime;

#[test]
fn imported_version_seven_reconciles_after_append_sealing_and_restart_without_renumbering() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::with_version(DocumentVersion::new(7).unwrap());
    db.seed_case(source.case_id);
    let source_before = std::fs::read(source.document_path()).unwrap();
    let import = source.inspect();
    import.apply(&db.url).unwrap();
    let at = OffsetDateTime::now_utc();
    let actor = UserId::from_uuid(
        db.client
            .query_one(
                "SELECT created_by FROM cases WHERE id=$1",
                &[&source.case_id.as_uuid()],
            )
            .unwrap()
            .get(0),
    );
    let store = PostgresCaseDocumentStore::connect(&db.url).unwrap();
    let mut original = store
        .load(
            actor,
            source.case_id,
            source.id,
            VersionSelection::Only,
            DocumentAction::Verify,
        )
        .unwrap();
    let version = original.version.next().unwrap();
    let content = b"a genuinely new version";
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
    store
        .append(actor, source.case_id, original.version, later.clone(), at)
        .unwrap();
    original
        .seal(application::documents::SealedEvidence {
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
    let before = PostgresAuditLog::connect(&db.url)
        .unwrap()
        .load_all()
        .unwrap();
    source.inspect().check_target(&db.url).unwrap();
    source.inspect().apply(&db.url).unwrap();
    let runtime_url = db.restricted_url();
    infrastructure::legacy::require_completed_import(source.dir.path(), &runtime_url).unwrap();
    assert_eq!(
        PostgresAuditLog::connect(&db.url)
            .unwrap()
            .load_all()
            .unwrap(),
        before
    );
    assert_eq!(before[..source.entries.len()], source.entries);
    assert_eq!(
        std::fs::read(source.document_path()).unwrap(),
        source_before
    );
    let reopened = PostgresCaseDocumentStore::open(&runtime_url).unwrap();
    let history = reopened
        .history(
            actor,
            source.case_id,
            source.id,
            VersionQuery::new(10, None).unwrap(),
            at,
        )
        .unwrap();
    assert_eq!(
        history.first_available_version,
        DocumentVersion::new(7).unwrap()
    );
    assert_eq!(
        history
            .versions
            .iter()
            .map(|item| item.document.version.get())
            .collect::<Vec<_>>(),
        vec![8, 7]
    );
    assert_eq!(
        reopened
            .load(
                actor,
                source.case_id,
                source.id,
                VersionSelection::Exact(original.version),
                DocumentAction::Verify
            )
            .unwrap(),
        original
    );
    assert_eq!(
        reopened
            .load(
                actor,
                source.case_id,
                source.id,
                VersionSelection::Current,
                DocumentAction::Verify
            )
            .unwrap(),
        later
    );
}
