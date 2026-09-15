mod document_store_support;

use application::documents::{CaseDocumentStore, DocumentAction};
use application::ApplicationError;
use domain::identity::Role;
use infrastructure::{initialize_database, PostgresCaseDocumentStore};
use postgres::error::SqlState;
use postgres::{Client, NoTls};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

static PERMISSION_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn runtime_can_commit_documents_but_cannot_rewrite_history_or_case_binding() {
    let _guard = PERMISSION_TESTS.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Litigator);
    let case_id = case(&url, actor);
    let mut admin = Client::connect(&url, NoTls).unwrap();
    let role = format!("runtime_{}", uuid::Uuid::new_v4().simple());
    admin
        .batch_execute(&format!(
            "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'"
        ))
        .unwrap();
    initialize_database(&url, &role).unwrap();
    let mut runtime_url = reqwest::Url::parse(&url).unwrap();
    runtime_url.set_username(&role).unwrap();
    runtime_url.set_password(Some("runtime-test-only")).unwrap();
    let store = PostgresCaseDocumentStore::open(runtime_url.as_str()).unwrap();
    let mut record = document();
    store
        .insert(actor, case_id, record.clone(), OffsetDateTime::now_utc())
        .unwrap();
    record.seal(evidence()).unwrap();
    store
        .seal(actor, case_id, record.clone(), OffsetDateTime::now_utc())
        .unwrap();
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
    assert!(!store.audit_entries(owner).unwrap().is_empty());
    let mut runtime = Client::connect(runtime_url.as_str(), NoTls).unwrap();
    for query in [
        "UPDATE audit_events SET actor=actor WHERE false",
        "DELETE FROM audit_events WHERE false",
        "TRUNCATE audit_events",
        "UPDATE documents SET case_id=case_id WHERE false",
    ] {
        let mut transaction = runtime.transaction().unwrap();
        let result = transaction.batch_execute(query);
        transaction.rollback().unwrap();
        assert_eq!(
            result.unwrap_err().code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
    assert!(matches!(
        PostgresCaseDocumentStore::open(&url),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    admin
        .batch_execute(&format!("GRANT UPDATE(actor) ON audit_events TO {role}"))
        .unwrap();
    assert!(matches!(
        PostgresCaseDocumentStore::open(runtime_url.as_str()),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn rejected_runtime_role_does_not_leave_partial_grants() {
    let _guard = PERMISSION_TESTS.lock().unwrap();
    let Some(url) = database_url() else { return };
    PostgresCaseDocumentStore::connect(&url).unwrap();
    let mut admin = Client::connect(&url, NoTls).unwrap();
    let owner: String = admin.query_one("SELECT current_user", &[]).unwrap().get(0);
    let before: Option<String> = admin
        .query_one(
            "SELECT relacl::text FROM pg_class WHERE oid='audit_events'::regclass",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(matches!(
        initialize_database(&url, &owner),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    let after: Option<String> = admin
        .query_one(
            "SELECT relacl::text FROM pg_class WHERE oid='audit_events'::regclass",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(after, before);
}

#[test]
fn runtime_rejects_ownership_of_document_protection_and_import_receipts() {
    let _guard = PERMISSION_TESTS.lock().unwrap();
    let Some(url) = database_url() else { return };
    let (mut admin, role, runtime_url) = runtime_role(&url);
    let owner: String = admin
        .query_one("SELECT quote_ident(current_user)", &[])
        .unwrap()
        .get(0);
    for object in [
        "TABLE documents",
        "TABLE document_series",
        "TABLE document_metadata_revisions",
        "FUNCTION document_metadata_text_valid(text,integer)",
        "FUNCTION document_metadata_is_canonical(text,text,text[])",
        "FUNCTION document_metadata_bytes(text,text,text[])",
        "FUNCTION preserve_document_metadata()",
        "FUNCTION enforce_document_metadata_sequence()",
        "FUNCTION preserve_document_series()",
        "FUNCTION enforce_document_version_sequence()",
        "FUNCTION preserve_document_evidence()",
        "TABLE migration_receipts",
    ] {
        admin
            .batch_execute(&format!("ALTER {object} OWNER TO {role}"))
            .unwrap();
        let result = PostgresCaseDocumentStore::open(&runtime_url);
        admin
            .batch_execute(&format!("ALTER {object} OWNER TO {owner}"))
            .unwrap();
        assert!(
            matches!(result, Err(ApplicationError::InvalidConfiguration(_))),
            "runtime must not own {object}"
        );
    }
    let inherited_owner = format!("object_owner_{}", uuid::Uuid::new_v4().simple());
    admin
        .batch_execute(&format!(
            "CREATE ROLE {inherited_owner} NOLOGIN;
             ALTER TABLE documents OWNER TO {inherited_owner};
             GRANT {inherited_owner} TO {role}"
        ))
        .unwrap();
    let result = PostgresCaseDocumentStore::open(&runtime_url);
    admin
        .batch_execute(&format!("ALTER TABLE documents OWNER TO {owner}"))
        .unwrap();
    assert!(matches!(
        result,
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn runtime_rejects_extra_document_receipt_and_schema_privileges() {
    let _guard = PERMISSION_TESTS.lock().unwrap();
    let Some(url) = database_url() else { return };
    let (mut admin, role, runtime_url) = runtime_role(&url);
    for privilege in [
        "UPDATE(id) ON document_series",
        "UPDATE(case_id) ON document_series",
        "UPDATE(first_available_version) ON document_series",
        "UPDATE ON document_series",
        "DELETE ON document_series",
        "TRUNCATE ON document_series",
        "TRIGGER ON document_series",
        "UPDATE(id) ON documents",
        "UPDATE(case_id) ON documents",
        "UPDATE(version) ON documents",
        "UPDATE(name) ON documents",
        "UPDATE(digest) ON documents",
        "UPDATE(vault) ON documents",
        "UPDATE ON documents",
        "DELETE ON documents",
        "TRUNCATE ON documents",
        "TRIGGER ON documents",
        "INSERT ON migration_receipts",
        "INSERT(fingerprint) ON migration_receipts",
        "UPDATE(fingerprint) ON migration_receipts",
        "UPDATE ON migration_receipts",
        "DELETE ON migration_receipts",
        "TRUNCATE ON migration_receipts",
        "TRIGGER ON migration_receipts",
        "CREATE ON SCHEMA public",
    ] {
        admin
            .batch_execute(&format!("GRANT {privilege} TO {role}"))
            .unwrap();
        let result = PostgresCaseDocumentStore::open(&runtime_url);
        admin
            .batch_execute(&format!("REVOKE {privilege} FROM {role}"))
            .unwrap();
        assert!(
            matches!(result, Err(ApplicationError::InvalidConfiguration(_))),
            "runtime must reject {privilege}"
        );
    }
}

#[test]
fn runtime_rejects_a_writable_schema_before_the_catalog_in_its_search_path() {
    let _guard = PERMISSION_TESTS.lock().unwrap();
    let Some(url) = database_url() else { return };
    let (mut admin, role, runtime_url) = runtime_role(&url);
    let schema = format!("shadow_{}", uuid::Uuid::new_v4().simple());
    admin
        .batch_execute(&format!(
            "CREATE SCHEMA {schema} AUTHORIZATION {role};
             ALTER ROLE {role} SET search_path={schema}, public, pg_catalog;
             CREATE FUNCTION {schema}.pg_has_role(oid, oid, text) RETURNS boolean
                 LANGUAGE sql AS 'SELECT false';
             CREATE FUNCTION {schema}.false_oid_comparison(oid, oid) RETURNS boolean
                 LANGUAGE sql AS 'SELECT false';
             CREATE OPERATOR {schema}.= (LEFTARG=oid, RIGHTARG=oid,
                 FUNCTION={schema}.false_oid_comparison);
             CREATE FUNCTION {schema}.false_schema_comparison(name, name) RETURNS boolean
                 LANGUAGE sql AS 'SELECT $1::text LIKE ''runtime_%''';
             CREATE OPERATOR {schema}.= (LEFTARG=name, RIGHTARG=name,
                 FUNCTION={schema}.false_schema_comparison)"
        ))
        .unwrap();
    assert!(matches!(
        PostgresCaseDocumentStore::open(&runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn runtime_rejects_a_write_role_it_can_assume_without_inheriting() {
    let _guard = PERMISSION_TESTS.lock().unwrap();
    let Some(url) = database_url() else { return };
    let (mut admin, role, runtime_url) = runtime_role(&url);
    let writer = format!("writer_{}", uuid::Uuid::new_v4().simple());
    admin
        .batch_execute(&format!(
            "CREATE ROLE {writer} NOLOGIN;
             GRANT UPDATE(first_available_version) ON document_series TO {writer};
             ALTER ROLE {role} NOINHERIT;
             GRANT {writer} TO {role}"
        ))
        .unwrap();
    let mut runtime = Client::connect(&runtime_url, NoTls).unwrap();
    let check = "SELECT has_column_privilege(current_user, 'document_series', 'first_available_version', 'UPDATE')";
    assert!(!runtime.query_one(check, &[]).unwrap().get::<_, bool>(0));
    runtime
        .batch_execute(&format!("SET ROLE {writer}"))
        .unwrap();
    assert!(runtime.query_one(check, &[]).unwrap().get::<_, bool>(0));
    assert!(matches!(
        PostgresCaseDocumentStore::open(&runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

fn runtime_role(url: &str) -> (Client, String, String) {
    PostgresCaseDocumentStore::connect(url).unwrap();
    let mut admin = Client::connect(url, NoTls).unwrap();
    let role = format!("runtime_{}", uuid::Uuid::new_v4().simple());
    admin
        .batch_execute(&format!(
            "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'"
        ))
        .unwrap();
    initialize_database(url, &role).unwrap();
    let mut runtime_url = reqwest::Url::parse(url).unwrap();
    runtime_url.set_username(&role).unwrap();
    runtime_url.set_password(Some("runtime-test-only")).unwrap();
    (admin, role, runtime_url.to_string())
}
