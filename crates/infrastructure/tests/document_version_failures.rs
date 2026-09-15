mod document_store_support;

use application::documents::{CaseDocumentStore, DocumentAction, VersionQuery, VersionSelection};
use application::ApplicationError;
use domain::identity::Role;
use infrastructure::PostgresCaseDocumentStore;
use postgres::{Client, NoTls};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

#[test]
fn failed_version_insert_audit_and_deferred_commit_preserve_every_prior_snapshot() {
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let mut admin = Client::connect(&url, NoTls).unwrap();
    let at = OffsetDateTime::now_utc();
    for deferred in [None, Some(false), Some(true)] {
        let mut first = document();
        store.insert(owner, case, first.clone(), at).unwrap();
        first.seal(evidence()).unwrap();
        store.seal(owner, case, first.clone(), at).unwrap();
        let mut next = first.clone();
        next.version = first.version.next().unwrap();
        next.vault.push(2);
        next.evidence = None;
        let before = store.audit_entries(owner).unwrap();
        let suffix = first.id.as_uuid().simple();
        if let Some(deferred) = deferred {
            let (constraint, timing) = if deferred {
                ("CONSTRAINT ", "AFTER")
            } else {
                ("", "BEFORE")
            };
            let defer = if deferred {
                "DEFERRABLE INITIALLY DEFERRED"
            } else {
                ""
            };
            admin.batch_execute(&format!("CREATE FUNCTION reject_version_{suffix}() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected version failure'; END $$; CREATE {constraint}TRIGGER reject_version_{suffix} {timing} INSERT ON documents {defer} FOR EACH ROW WHEN (NEW.id='{}' AND NEW.version=2) EXECUTE FUNCTION reject_version_{suffix}()",first.id)).unwrap();
        } else {
            admin.batch_execute(&format!("ALTER TABLE audit_events ADD CONSTRAINT reject_version_{suffix} CHECK(resource NOT LIKE '%{}:version:2:%')",first.id)).unwrap();
        }
        let result = store.append(owner, case, first.version, next, at);
        if deferred.is_some() {
            admin.batch_execute(&format!("DROP TRIGGER reject_version_{suffix} ON documents; DROP FUNCTION reject_version_{suffix}()")).unwrap();
        } else {
            admin
                .batch_execute(&format!(
                    "ALTER TABLE audit_events DROP CONSTRAINT reject_version_{suffix}"
                ))
                .unwrap();
        }
        assert!(matches!(result, Err(ApplicationError::Port(_))));
        assert_eq!(
            store
                .load(
                    owner,
                    case,
                    first.id,
                    VersionSelection::Only,
                    DocumentAction::Verify
                )
                .unwrap(),
            first
        );
        assert_eq!(store.audit_entries(owner).unwrap(), before);
        assert_eq!(
            admin
                .query_one(
                    "SELECT COUNT(*) FROM documents WHERE id=$1",
                    &[&first.id.as_uuid()]
                )
                .unwrap()
                .get::<_, i64>(0),
            1
        );
    }
    let record = document();
    store.insert(owner, case, record.clone(), at).unwrap();
    let suffix = record.id.as_uuid().simple();
    admin.batch_execute(&format!("ALTER TABLE audit_events ADD CONSTRAINT reject_history_{suffix} CHECK(resource<>'case:{case}:document:{}:versions')",record.id)).unwrap();
    let before = store.audit_entries(owner).unwrap();
    let result = store.history(
        owner,
        case,
        record.id,
        VersionQuery::new(10, None).unwrap(),
        at,
    );
    admin
        .batch_execute(&format!(
            "ALTER TABLE audit_events DROP CONSTRAINT reject_history_{suffix}"
        ))
        .unwrap();
    assert!(matches!(result, Err(ApplicationError::Port(_))));
    assert_eq!(store.audit_entries(owner).unwrap(), before);
}
