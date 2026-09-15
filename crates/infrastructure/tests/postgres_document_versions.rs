mod document_store_support;

use application::cases::CaseRepository;
use application::documents::{
    CaseDocumentStore, DocumentAction, DocumentQuery, DocumentRecord, VersionQuery,
    VersionSelection,
};
use application::ApplicationError;
use domain::crypto::DocumentVersion;
use domain::identity::Role;
use infrastructure::{PostgresCaseDocumentStore, PostgresCaseRepository};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

static DATABASE_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn next(record: &DocumentRecord) -> DocumentRecord {
    let mut next = record.clone();
    next.version = record.version.next().unwrap();
    next.name = format!("version-{}.txt", next.version.get());
    next.vault.push(next.version.get() as u8);
    next.evidence = None;
    next
}

#[test]
fn append_preserves_history_and_filters_only_the_current_snapshot() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let at = OffsetDateTime::now_utc();
    let mut first = document();
    store.insert(owner, case, first.clone(), at).unwrap();
    first.seal(evidence()).unwrap();
    store.seal(owner, case, first.clone(), at).unwrap();
    let second = next(&first);
    store
        .append(owner, case, first.version, second.clone(), at)
        .unwrap();
    assert_eq!(
        store
            .load(
                owner,
                case,
                first.id,
                VersionSelection::Exact(first.version),
                DocumentAction::Verify
            )
            .unwrap(),
        first
    );
    assert_eq!(
        store
            .get(owner, case, first.id, VersionSelection::Current, at)
            .unwrap()
            .document
            .version,
        second.version
    );
    assert!(matches!(
        store.load(
            owner,
            case,
            first.id,
            VersionSelection::Only,
            DocumentAction::Verify
        ),
        Err(ApplicationError::DocumentVersionRequired)
    ));
    for query in [
        DocumentQuery::new(10, 0, Some(&first.name), None).unwrap(),
        DocumentQuery::new(10, 0, None, Some(true)).unwrap(),
    ] {
        assert!(store
            .list(owner, case, query, at)
            .unwrap()
            .documents
            .is_empty());
    }
    let pending = store
        .list(
            owner,
            case,
            DocumentQuery::new(10, 0, None, Some(false)).unwrap(),
            at,
        )
        .unwrap();
    assert_eq!(pending.documents.len(), 1);
    assert_eq!(
        pending.documents[0].content.document.version,
        second.version
    );
    let page = store
        .history(
            owner,
            case,
            first.id,
            VersionQuery::new(1, None).unwrap(),
            at,
        )
        .unwrap();
    assert_eq!(page.versions[0].document.version, second.version);
    assert!(page.has_more);
    assert_eq!(page.next_before_version, Some(second.version));
    assert_eq!(page.first_available_version, first.version);
    let third = next(&second);
    store
        .append(owner, case, second.version, third, at)
        .unwrap();
    let older = store
        .history(
            owner,
            case,
            first.id,
            VersionQuery::new(1, Some(second.version.get())).unwrap(),
            at,
        )
        .unwrap();
    assert_eq!(older.versions.len(), 1);
    assert_eq!(older.versions[0].document.version, first.version);
    assert!(older.versions[0].document.sealed);
    assert!(!older.has_more);
    assert_eq!(older.next_before_version, None);
    store
        .record_access(owner, case, &first, DocumentAction::Export, at)
        .unwrap();
    let resource = format!(
        "case:{case}:document:{}:version:1:sha256:{}",
        first.id,
        first.digest.to_hex()
    );
    assert!(store
        .audit_entries(owner)
        .unwrap()
        .iter()
        .any(|entry| entry.event.action == "document.evidence_exported"
            && entry.event.resource == resource));
}

#[test]
fn competing_appends_have_one_winner_and_sealing_the_previous_version_is_independent() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let at = OffsetDateTime::now_utc();
    let first = document();
    store.insert(owner, case, first.clone(), at).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(5));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let adapter = PostgresCaseDocumentStore::connect(&url).unwrap();
            let barrier = barrier.clone();
            let mut candidate = next(&first);
            candidate.vault.push(index);
            std::thread::spawn(move || {
                barrier.wait();
                let result = adapter.append(
                    owner,
                    case,
                    DocumentVersion::initial(),
                    candidate.clone(),
                    at,
                );
                (result, candidate)
            })
        })
        .collect();
    let mut sealed = first.clone();
    sealed.seal(evidence()).unwrap();
    barrier.wait();
    store.seal(owner, case, sealed.clone(), at).unwrap();
    let mut winner = None;
    for worker in workers {
        let (result, candidate) = worker.join().unwrap();
        match result {
            Ok(overview) => {
                assert_eq!(overview.content.document.version, candidate.version);
                assert_eq!(overview.current_metadata.metadata_revision.get(), 0);
                assert!(winner.replace(candidate).is_none());
            }
            Err(ApplicationError::DocumentVersionConflict) => {}
            other => panic!("unexpected append result: {other:?}"),
        }
    }
    assert_eq!(
        store
            .load(
                owner,
                case,
                first.id,
                VersionSelection::Current,
                DocumentAction::Verify
            )
            .unwrap(),
        winner.unwrap()
    );
    assert_eq!(
        store
            .load(
                owner,
                case,
                first.id,
                VersionSelection::Exact(first.version),
                DocumentAction::Verify
            )
            .unwrap(),
        sealed
    );
    let entries = store.audit_entries(owner).unwrap();
    for action in ["document.version_added", "document.sealed"] {
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.event.action == action
                    && entry.event.resource.contains(&first.id.to_string()))
                .count(),
            1
        );
    }
}

#[test]
fn append_and_history_recheck_roles_membership_and_exact_case_before_version_conflicts() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Paralegal);
    let client = user(&url, Role::Client);
    let case = case(&url, owner);
    let foreign = document_store_support::case(&url, owner);
    let cases = PostgresCaseRepository::connect(&url).unwrap();
    cases.add_member(case, actor, owner).unwrap();
    cases.add_member(case, client, owner).unwrap();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let at = OffsetDateTime::now_utc();
    let first = document();
    store.insert(actor, case, first.clone(), at).unwrap();
    let second = next(&first);
    store
        .append(actor, case, first.version, second.clone(), at)
        .unwrap();
    let before = store.audit_entries(owner).unwrap();
    assert!(matches!(
        store.append(actor, case, first.version, second.clone(), at),
        Err(ApplicationError::DocumentVersionConflict)
    ));
    assert!(matches!(
        store.append(owner, foreign, first.version, second.clone(), at),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        store.append(client, case, first.version, second.clone(), at),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        store.history(
            client,
            case,
            first.id,
            VersionQuery::new(10, None).unwrap(),
            at
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        store.get(
            owner,
            case,
            first.id,
            VersionSelection::Exact(DocumentVersion::new(8).unwrap()),
            at
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert_eq!(store.audit_entries(owner).unwrap(), before);
    cases.remove_member(case, actor, owner).unwrap();
    assert!(matches!(
        store.append(actor, case, second.version, next(&second), at),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        store.history(
            actor,
            case,
            first.id,
            VersionQuery::new(10, None).unwrap(),
            at
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
}
