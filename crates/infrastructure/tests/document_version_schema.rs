#[allow(dead_code)]
mod version_database_support;

use application::ApplicationError;
use infrastructure::{initialize_database, PostgresCaseDocumentStore};
use uuid::Uuid;
use version_database_support::Database;

#[test]
fn an_unmigrated_runtime_schema_reports_the_required_administrative_migration() {
    let Some(db) = Database::old_schema() else {
        return;
    };
    let result = PostgresCaseDocumentStore::open(&db.url);
    assert!(
        matches!(result,Err(ApplicationError::InvalidConfiguration(ref message)) if message.contains("migrate"))
    );
}

#[test]
fn startup_rejects_incomplete_version_inventory_and_disabled_constraints() {
    for corruption in [
        "SET session_replication_role=replica; DELETE FROM document_series; SET session_replication_role=origin",
        "SET session_replication_role=replica; DELETE FROM documents WHERE version=1; SET session_replication_role=origin",
        "SET session_replication_role=replica; DELETE FROM documents WHERE version=2; SET session_replication_role=origin",
        "ALTER TABLE documents DISABLE TRIGGER documents_version_sequence",
        "ALTER TABLE document_series DISABLE TRIGGER document_series_preserve_identity",
        "ALTER TABLE documents DROP CONSTRAINT documents_series_case_fk",
        "ALTER TABLE documents DROP CONSTRAINT documents_series_case_fk; ALTER TABLE documents ADD CONSTRAINT documents_series_case_fk FOREIGN KEY(id,case_id) REFERENCES document_series(id,case_id) NOT VALID",
        "DO $$ DECLARE name text; BEGIN SELECT t.tgname INTO name FROM pg_trigger t JOIN pg_constraint c ON c.oid=t.tgconstraint WHERE c.conname='documents_series_case_fk' AND t.tgrelid='documents'::regclass LIMIT 1; EXECUTE format('ALTER TABLE documents DISABLE TRIGGER %I',name); END $$",
    ] {
        let Some(mut db) = Database::old_schema() else { return };
        let case = db.seed_case();
        let id = Uuid::new_v4();
        db.client.execute("INSERT INTO documents VALUES($1,$2,1,'first.txt',$3,$4,NULL)", &[&id,&case,&vec![3_u8;32],&vec![7_u8;80]]).unwrap();
        let role = format!("schema_runtime_{}",Uuid::new_v4().simple());
        db.control.batch_execute(&format!("CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'")).unwrap();
        initialize_database(&db.url,&role).unwrap();
        for version in [2_i64,3] {
            db.client.execute("INSERT INTO documents VALUES($1,$2,$3,'next.txt',$4,$5,NULL)", &[&id,&case,&version,&vec![3_u8;32],&vec![7_u8;80]]).unwrap();
        }
        let mut runtime = reqwest::Url::parse(&db.url).unwrap();
        runtime.set_username(&role).unwrap();
        runtime.set_password(Some("runtime-test-only")).unwrap();
        PostgresCaseDocumentStore::open(runtime.as_str()).unwrap();
        db.client.batch_execute(corruption).unwrap();
        assert!(matches!(PostgresCaseDocumentStore::open(runtime.as_str()),Err(ApplicationError::InvalidConfiguration(_))), "accepted {corruption}");
        db.control.batch_execute(&format!("DROP SCHEMA {} CASCADE; DROP ROLE {role}",db.schema)).unwrap();
    }
}
