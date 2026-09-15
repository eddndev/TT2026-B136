mod document_store_support;

use application::cases::CaseRepository;
use application::documents::{CaseDocumentStore, DocumentAction};
use application::ApplicationError;
use domain::identity::Role;
use infrastructure::{PostgresCaseDocumentStore, PostgresCaseRepository};
use postgres::{Client, NoTls};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

static FAILURE_INJECTION: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn a_deferred_commit_failure_rolls_back_both_document_and_success_event() {
    let _guard = FAILURE_INJECTION.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case_id = case(&url, owner);
    let record = document();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let mut admin = Client::connect(&url, NoTls).unwrap();
    let suffix = record.id.as_uuid().simple().to_string();
    admin
        .batch_execute(&format!(
            "CREATE FUNCTION reject_commit_{suffix}() RETURNS TRIGGER LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'injected document commit failure'; END; $$;
         CREATE CONSTRAINT TRIGGER reject_commit_{suffix} AFTER INSERT ON documents
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.id='{}')
         EXECUTE FUNCTION reject_commit_{suffix}();",
            record.id,
        ))
        .unwrap();
    let result = store.insert(owner, case_id, record.clone(), OffsetDateTime::now_utc());
    admin.batch_execute(&format!("DROP TRIGGER reject_commit_{suffix} ON documents; DROP FUNCTION reject_commit_{suffix}();")).unwrap();
    assert!(matches!(result, Err(ApplicationError::Port(_))));
    assert!(matches!(
        store.load(
            owner,
            case_id,
            record.id,
            application::documents::VersionSelection::Only,
            DocumentAction::Verify
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(!store
        .audit_entries(owner)
        .unwrap()
        .iter()
        .any(|entry| entry.event.resource.contains(&record.id.to_string())));
}

#[test]
fn an_audit_failure_preserves_both_prior_evidence_state_and_membership() {
    let _guard = FAILURE_INJECTION.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Litigator);
    let case_id = case(&url, actor);
    let record = document();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let cases = PostgresCaseRepository::connect(
        &url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    store
        .insert(actor, case_id, record.clone(), OffsetDateTime::now_utc())
        .unwrap();
    let mut prepared = record.clone();
    prepared.seal(evidence()).unwrap();
    let mut admin = Client::connect(&url, NoTls).unwrap();
    let cutoff: i64 = admin
        .query_one("SELECT COALESCE(MAX(sequence)+1,0) FROM audit_events", &[])
        .unwrap()
        .get(0);
    let constraint = format!("reject_changes_{}", case_id.as_uuid().simple());
    admin.batch_execute(&format!("ALTER TABLE audit_events ADD CONSTRAINT {constraint} CHECK(sequence < {cutoff} OR resource NOT LIKE 'case:{case_id}:%')")).unwrap();
    let seal_result = store.seal(actor, case_id, prepared, OffsetDateTime::now_utc());
    let remove_result = cases.remove_member(case_id, actor, owner, time::OffsetDateTime::now_utc());
    admin
        .batch_execute(&format!(
            "ALTER TABLE audit_events DROP CONSTRAINT {constraint}"
        ))
        .unwrap();
    assert!(matches!(seal_result, Err(ApplicationError::Port(_))));
    assert!(matches!(remove_result, Err(ApplicationError::Port(_))));
    assert_eq!(
        store
            .load(
                actor,
                case_id,
                record.id,
                application::documents::VersionSelection::Only,
                DocumentAction::Verify
            )
            .unwrap(),
        record
    );
    assert!(cases
        .get_basic(actor, case_id, time::OffsetDateTime::now_utc())
        .is_ok());
}
