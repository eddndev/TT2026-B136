mod document_store_support;

use application::cases::CaseRepository;
use application::documents::{CaseDocumentStore, DocumentQuery, DocumentSummary};
use application::ApplicationError;
use domain::audit::{verify_chain, ChainVerification};
use domain::cases::CaseId;
use domain::crypto::DocumentId;
use domain::identity::Role;
use infrastructure::{PostgresCaseDocumentStore, PostgresCaseRepository, RingSha256Hasher};
use postgres::{Client, NoTls};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

// One fixture changes audit constraints to exercise a failed access commit.
static DATABASE_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn query(limit: u32, offset: u32, name: Option<&str>, sealed: Option<bool>) -> DocumentQuery {
    DocumentQuery::new(limit, offset, name, sealed).unwrap()
}

#[test]
fn pagination_filters_case_name_and_sealed_state_before_counting_results() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Litigator);
    let case_id = case(&url, actor);
    let foreign_case = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let at = OffsetDateTime::now_utc();
    let mut matching = Vec::new();
    for (name, sealed) in [
        ("Proof_100.PDF", false),
        ("proof_100.txt", false),
        ("PROOF_100.pdf", true),
        ("proofA100.pdf", false),
        ("other.txt", false),
    ] {
        let mut record = document();
        record.name = name.into();
        store.insert(actor, case_id, record.clone(), at).unwrap();
        if sealed {
            record.seal(evidence()).unwrap();
            store.seal(actor, case_id, record.clone(), at).unwrap();
        }
        if name.to_lowercase().contains("proof_100") && !sealed {
            matching.push(record.id);
        }
    }
    let mut foreign = document();
    foreign.name = "Proof_100.PDF".into();
    store.insert(owner, foreign_case, foreign, at).unwrap();
    matching.sort_by_key(|id| id.as_uuid());
    for (offset, expected, more) in [
        (0, Some(matching[0]), true),
        (1, Some(matching[1]), false),
        (2, None, false),
    ] {
        let page = store
            .list(
                actor,
                case_id,
                query(1, offset, Some("PROOF_100"), Some(false)),
                at,
            )
            .unwrap();
        assert_eq!(
            page.documents
                .iter()
                .map(|item| item.content.document.id)
                .collect::<Vec<_>>(),
            expected.into_iter().collect::<Vec<_>>()
        );
        assert!(page
            .documents
            .iter()
            .all(|item| item.content.case_id == case_id));
        assert_eq!(page.has_more, more);
    }
    for literal in ["%", "\\"] {
        let page = store
            .list(actor, case_id, query(100, 0, Some(literal), None), at)
            .unwrap();
        assert!(page.documents.is_empty());
        assert!(!page.has_more);
    }
    let sealed = store
        .list(actor, case_id, query(100, 0, None, Some(true)), at)
        .unwrap();
    assert_eq!(sealed.documents.len(), 1);
    assert!(sealed.documents[0].content.document.sealed);
    let all = store
        .list(actor, case_id, query(100, 0, None, None), at)
        .unwrap();
    assert_eq!(all.documents.len(), 5);
    assert!(!all.has_more);
    assert!(all
        .documents
        .windows(2)
        .all(|pair| pair[0].content.document.id.as_uuid() < pair[1].content.document.id.as_uuid()));
    assert!(store
        .list(actor, case_id, query(1, u32::MAX, None, None), at)
        .unwrap()
        .documents
        .is_empty());
}

#[test]
fn detail_and_list_return_metadata_without_decoding_encrypted_or_sealed_material() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case_id = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let record = document();
    let at = OffsetDateTime::from_unix_timestamp(1_780_000_000).unwrap();
    store.insert(owner, case_id, record.clone(), at).unwrap();
    Client::connect(&url, NoTls)
        .unwrap()
        .execute(
            "UPDATE documents SET evidence='{}'::jsonb WHERE id=$1",
            &[&record.id.as_uuid()],
        )
        .unwrap();
    let result = store
        .get(
            owner,
            case_id,
            record.id,
            application::documents::VersionSelection::Current,
            at,
        )
        .unwrap();
    let mut expected = DocumentSummary::from(&record);
    expected.sealed = true;
    assert_eq!(result.document, expected);
    assert_eq!(result.case_id, case_id);
    assert_eq!(
        store
            .list(owner, case_id, query(1, 0, None, None), at)
            .unwrap()
            .documents
            .into_iter()
            .map(|item| item.content)
            .collect::<Vec<_>>(),
        vec![result]
    );
    let entries = store.audit_entries(owner).unwrap();
    for (action, resource) in [
        (
            "document.read",
            format!(
                "case:{case_id}:document:{}:version:{}:sha256:{}",
                record.id,
                record.version.get(),
                record.digest.to_hex()
            ),
        ),
        ("document.listed", format!("case:{case_id}:documents")),
    ] {
        let event = &entries
            .iter()
            .find(|entry| entry.event.action == action && entry.event.resource == resource)
            .unwrap()
            .event;
        assert_eq!(event.timestamp, at);
        assert_eq!(event.actor, format!("{owner}@example.com"));
    }
    assert!(matches!(
        verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
        ChainVerification::Valid { .. }
    ));
}

#[test]
fn unauthorized_missing_and_foreign_documents_are_indistinguishable() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Paralegal);
    let case_id = case(&url, owner);
    let foreign_case = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let record = document();
    let at = OffsetDateTime::now_utc();
    store.insert(owner, case_id, record.clone(), at).unwrap();
    let cases = PostgresCaseRepository::connect(&url).unwrap();
    cases.add_member(foreign_case, actor, owner).unwrap();
    for (user, scope, id) in [
        (owner, foreign_case, record.id),
        (owner, case_id, DocumentId::new()),
        (actor, case_id, record.id),
        (actor, CaseId::new(), record.id),
    ] {
        assert!(matches!(
            store.get(
                user,
                scope,
                id,
                application::documents::VersionSelection::Current,
                at
            ),
            Err(ApplicationError::DocumentNotFound(_))
        ));
    }
    for scope in [case_id, CaseId::new()] {
        assert!(matches!(
            store.list(actor, scope, query(20, 0, None, None), at),
            Err(ApplicationError::CaseNotFound)
        ));
    }
    let empty = store
        .list(actor, foreign_case, query(20, 0, None, None), at)
        .unwrap();
    assert!(empty.documents.is_empty());
    assert!(!empty.has_more);
}

#[test]
fn reads_reload_current_membership_role_and_active_status() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Paralegal);
    let client = user(&url, Role::Client);
    let case_id = case(&url, owner);
    let cases = PostgresCaseRepository::connect(&url).unwrap();
    cases.add_member(case_id, actor, owner).unwrap();
    cases.add_member(case_id, client, owner).unwrap();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let record = document();
    let at = OffsetDateTime::now_utc();
    store.insert(owner, case_id, record.clone(), at).unwrap();
    store
        .get(
            actor,
            case_id,
            record.id,
            application::documents::VersionSelection::Current,
            at,
        )
        .unwrap();
    store
        .list(actor, case_id, query(20, 0, None, None), at)
        .unwrap();
    assert!(matches!(
        store.get(
            client,
            case_id,
            record.id,
            application::documents::VersionSelection::Current,
            at
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        store.list(client, case_id, query(20, 0, None, None), at),
        Err(ApplicationError::PermissionDenied)
    ));
    cases.remove_member(case_id, actor, owner).unwrap();
    assert!(matches!(
        store.get(
            actor,
            case_id,
            record.id,
            application::documents::VersionSelection::Current,
            at
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        store.list(actor, case_id, query(20, 0, None, None), at),
        Err(ApplicationError::CaseNotFound)
    ));
    cases.add_member(case_id, actor, owner).unwrap();
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin
        .execute(
            "UPDATE users SET role='client' WHERE id=$1",
            &[&actor.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.get(
            actor,
            case_id,
            record.id,
            application::documents::VersionSelection::Current,
            at
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        store.list(actor, case_id, query(20, 0, None, None), at),
        Err(ApplicationError::PermissionDenied)
    ));
    admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&actor.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.get(
            actor,
            case_id,
            record.id,
            application::documents::VersionSelection::Current,
            at
        ),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        store.list(actor, case_id, query(20, 0, None, None), at),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn audit_failure_prevents_detail_and_list_results_without_corrupting_the_chain() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case_id = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let record = document();
    let at = OffsetDateTime::now_utc();
    store.insert(owner, case_id, record.clone(), at).unwrap();
    let before = store.audit_entries(owner).unwrap();
    let constraint = format!("reject_metadata_{}", record.id.as_uuid().simple());
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin.batch_execute(&format!("ALTER TABLE audit_events ADD CONSTRAINT {constraint} CHECK (action NOT IN ('document.read','document.listed') OR resource NOT LIKE 'case:{case_id}:%')")).unwrap();
    let detail = store.get(
        owner,
        case_id,
        record.id,
        application::documents::VersionSelection::Current,
        at,
    );
    let list = store.list(owner, case_id, query(20, 0, None, None), at);
    admin
        .batch_execute(&format!(
            "ALTER TABLE audit_events DROP CONSTRAINT {constraint}"
        ))
        .unwrap();
    assert!(matches!(detail, Err(ApplicationError::Port(_))));
    assert!(matches!(list, Err(ApplicationError::Port(_))));
    assert_eq!(store.audit_entries(owner).unwrap(), before);
    assert_eq!(
        store
            .get(
                owner,
                case_id,
                record.id,
                application::documents::VersionSelection::Current,
                at
            )
            .unwrap()
            .document,
        DocumentSummary::from(&record)
    );
}
