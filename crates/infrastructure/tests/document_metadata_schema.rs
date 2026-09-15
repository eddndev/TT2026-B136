#[allow(dead_code)]
mod version_database_support;

use application::ApplicationError;
use infrastructure::{initialize_database, PostgresCaseDocumentStore};
use version_database_support::Database;

#[test]
fn runtime_requires_metadata_constraints_and_active_triggers() {
    for corruption in [
        "DROP TABLE document_metadata_revisions",
        "ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_canonical",
        "ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_digest",
        "ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_series_fk",
        "ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_actor_fk",
        "ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_revisions_pkey",
        "ALTER TABLE document_metadata_revisions DISABLE TRIGGER document_metadata_sequence",
        "ALTER TABLE document_metadata_revisions DISABLE TRIGGER document_metadata_preserve_history",
        "ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_digest; ALTER TABLE document_metadata_revisions ADD CONSTRAINT document_metadata_digest CHECK (metadata_digest=pg_catalog.sha256(document_metadata_bytes(document_type,classification,tags))) NOT VALID",
        "DO $$ DECLARE name text; BEGIN SELECT t.tgname INTO name FROM pg_trigger t JOIN pg_constraint c ON c.oid=t.tgconstraint WHERE c.conname='document_metadata_actor_fk' AND t.tgrelid='document_metadata_revisions'::regclass LIMIT 1; EXECUTE format('ALTER TABLE document_metadata_revisions DISABLE TRIGGER %I',name); END $$",
    ] {
        let Some(mut db) = Database::old_schema() else { return };
        let (role,url) = runtime(&mut db);
        PostgresCaseDocumentStore::open(&url).unwrap();
        db.client.batch_execute(corruption).unwrap();
        assert!(matches!(PostgresCaseDocumentStore::open(&url),Err(ApplicationError::InvalidConfiguration(_))),"accepted {corruption}");
        db.control.batch_execute(&format!("DROP SCHEMA {} CASCADE; DROP ROLE {role}", db.schema)).unwrap();
    }
}

#[test]
fn database_encoding_must_preserve_unicode_before_migration_or_runtime_open() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let name = format!("metadata_latin1_{}", uuid::Uuid::new_v4().simple());
    db.control.batch_execute(&format!("CREATE DATABASE {name} TEMPLATE template0 ENCODING 'LATIN1' LC_COLLATE 'C' LC_CTYPE 'C' LOCALE_PROVIDER libc")).unwrap();
    let mut url = reqwest::Url::parse(&db.url).unwrap();
    url.set_path(&name);
    url.set_query(None);
    for result in [
        PostgresCaseDocumentStore::connect(url.as_str()),
        PostgresCaseDocumentStore::open(url.as_str()),
    ] {
        assert!(
            matches!(result,Err(ApplicationError::InvalidConfiguration(ref message)) if message.contains("UTF8"))
        );
    }
    let mut client = postgres::Client::connect(url.as_str(), postgres::NoTls).unwrap();
    assert!(!client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public')",
            &[]
        )
        .unwrap()
        .get::<_, bool>(0));
    drop(client);
    db.control
        .batch_execute(&format!("DROP DATABASE {name}"))
        .unwrap();
}

#[test]
fn startup_rejects_missing_first_or_intermediate_metadata_revisions() {
    for removed in [1_i64, 2] {
        let Some(mut db) = Database::old_schema() else {
            return;
        };
        let case = db.seed_case();
        let id = uuid::Uuid::new_v4();
        db.client
            .execute(
                "INSERT INTO documents VALUES($1,$2,1,'first.txt',$3,$4,NULL)",
                &[&id, &case, &vec![3_u8; 32], &vec![7_u8; 80]],
            )
            .unwrap();
        let (role, url) = runtime(&mut db);
        for revision in 1_i64..=3 {
            db.client.execute("INSERT INTO document_metadata_revisions(document_id,metadata_revision,tags,metadata_digest,changed_at,changed_by,changed_by_email) SELECT $1,$2,ARRAY[]::text[],pg_catalog.sha256(document_metadata_bytes(NULL,NULL,ARRAY[]::text[])),'2025-01-01T00:00:00Z',created_by,'author@example.test' FROM cases WHERE id=$3",&[&id,&revision,&case]).unwrap();
        }
        PostgresCaseDocumentStore::open(&url).unwrap();
        db.client
            .execute(
                "DELETE FROM document_metadata_revisions WHERE metadata_revision=$1",
                &[&removed],
            )
            .unwrap();
        assert!(matches!(
            PostgresCaseDocumentStore::open(&url),
            Err(ApplicationError::InvalidConfiguration(_))
        ));
        db.control
            .batch_execute(&format!(
                "DROP SCHEMA {} CASCADE; DROP ROLE {role}",
                db.schema
            ))
            .unwrap();
    }
}

#[test]
fn runtime_can_execute_metadata_checks_but_cannot_rewrite_history() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let (role, url) = runtime(&mut db);
    let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    assert_eq!(client.query_one("SELECT encode(pg_catalog.sha256(document_metadata_bytes(NULL,NULL,ARRAY[]::text[])),'hex')",&[]).unwrap().get::<_,String>(0), "adbad13daff0fc70b3309e1e58a20aecadc00077dac6f2a04349c4001e8443bb");
    for privilege in ["UPDATE", "UPDATE(tags)", "DELETE", "TRUNCATE", "TRIGGER"] {
        db.client
            .batch_execute(&format!(
                "GRANT {privilege} ON document_metadata_revisions TO {role}"
            ))
            .unwrap();
        let opened = PostgresCaseDocumentStore::open(&url);
        db.client
            .batch_execute(&format!(
                "REVOKE {privilege} ON document_metadata_revisions FROM {role}"
            ))
            .unwrap();
        assert!(
            matches!(opened, Err(ApplicationError::InvalidConfiguration(_))),
            "accepted {privilege}"
        );
    }
    drop(client);
    db.control
        .batch_execute(&format!(
            "DROP SCHEMA {} CASCADE; DROP ROLE {role}",
            db.schema
        ))
        .unwrap();
}

fn runtime(db: &mut Database) -> (String, String) {
    let role = format!("metadata_runtime_{}", uuid::Uuid::new_v4().simple());
    db.control
        .batch_execute(&format!(
            "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'"
        ))
        .unwrap();
    initialize_database(&db.url, &role).unwrap();
    let mut parsed = reqwest::Url::parse(&db.url).unwrap();
    parsed.set_username(&role).unwrap();
    parsed.set_password(Some("runtime-test-only")).unwrap();
    (role, parsed.to_string())
}
