#[allow(dead_code)]
mod version_database_support;

use application::documents::{
    CaseDocumentStore, DocumentAction, DocumentRecord, VersionQuery, VersionSelection,
};
use application::ApplicationError;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use domain::identity::UserId;
use infrastructure::PostgresCaseDocumentStore;
use time::OffsetDateTime;
use uuid::Uuid;
use version_database_support::Database;

#[test]
fn an_exhausted_imported_version_is_readable_but_cannot_append_or_write_an_event() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let case = db.seed_case();
    let id = Uuid::new_v4();
    db.client
        .execute(
            "INSERT INTO documents VALUES($1,$2,4294967295,'last.txt',$3,$4,NULL)",
            &[&id, &case, &vec![3_u8; 32], &vec![7_u8; 80]],
        )
        .unwrap();
    let actor = UserId::from_uuid(
        db.client
            .query_one("SELECT created_by FROM cases WHERE id=$1", &[&case])
            .unwrap()
            .get(0),
    );
    let case = domain::cases::CaseId::from_uuid(case);
    let id = DocumentId::from_uuid(id);
    let store = PostgresCaseDocumentStore::connect(&db.url).unwrap();
    let record = store
        .load(
            actor,
            case,
            id,
            VersionSelection::Only,
            DocumentAction::Verify,
        )
        .unwrap();
    let at = OffsetDateTime::now_utc();
    assert_eq!(record.version.get(), u32::MAX);
    let before = store.audit_entries(actor).unwrap();
    assert!(matches!(
        store.append(actor, case, record.version, record.clone(), at),
        Err(ApplicationError::DocumentVersionExhausted)
    ));
    assert_eq!(store.audit_entries(actor).unwrap(), before);
    let page = store
        .history(actor, case, id, VersionQuery::new(1, None).unwrap(), at)
        .unwrap();
    assert_eq!(page.first_available_version, record.version);
    assert_eq!(page.versions.len(), 1);
    assert!(!page.has_more);
    let empty = store
        .history(
            actor,
            case,
            id,
            VersionQuery::new(1, Some(u32::MAX)).unwrap(),
            at,
        )
        .unwrap();
    assert!(empty.versions.is_empty());
    assert_eq!(empty.first_available_version, record.version);
}

#[test]
fn invalid_appends_and_current_role_changes_do_not_mutate_document_history() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let case = db.seed_case();
    let actor = UserId::from_uuid(
        db.client
            .query_one("SELECT created_by FROM cases WHERE id=$1", &[&case])
            .unwrap()
            .get(0),
    );
    let case = domain::cases::CaseId::from_uuid(case);
    let store = PostgresCaseDocumentStore::connect(&db.url).unwrap();
    let record = DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "first.txt".into(),
        Sha256Digest::from_array([3; 32]),
        vec![7; 80],
    )
    .unwrap();
    let at = OffsetDateTime::now_utc();
    store.insert(actor, case, record.clone(), at).unwrap();
    let before = store.audit_entries(actor).unwrap();
    assert!(matches!(
        store.insert(actor, case, record.clone(), at),
        Err(ApplicationError::DocumentAlreadyExists(_))
    ));
    assert!(matches!(
        store.append(actor, case, record.version, record.clone(), at),
        Err(ApplicationError::InvalidInput(_))
    ));
    let mut next = record.clone();
    next.version = record.version.next().unwrap();
    assert!(matches!(
        store.insert(actor, case, next.clone(), at),
        Err(ApplicationError::InvalidInput(_))
    ));
    for sql in [
        "UPDATE users SET active=false WHERE id=$1",
        "UPDATE users SET active=true,role='client' WHERE id=$1",
    ] {
        db.client.execute(sql, &[&actor.as_uuid()]).unwrap();
        assert!(store
            .append(actor, case, record.version, next.clone(), at)
            .is_err());
        assert!(store
            .history(
                actor,
                case,
                record.id,
                VersionQuery::new(10, None).unwrap(),
                at
            )
            .is_err());
    }
    db.client
        .execute(
            "UPDATE users SET role='owner' WHERE id=$1",
            &[&actor.as_uuid()],
        )
        .unwrap();
    assert_eq!(store.audit_entries(actor).unwrap(), before);
    assert_eq!(
        store
            .load(
                actor,
                case,
                record.id,
                VersionSelection::Only,
                DocumentAction::Verify
            )
            .unwrap(),
        record
    );
}
