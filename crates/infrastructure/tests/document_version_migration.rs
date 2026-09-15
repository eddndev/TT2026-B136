use infrastructure::{initialize_database, PostgresCaseDocumentStore};
use postgres::{Client, NoTls};
use uuid::Uuid;

mod version_database_support;
use version_database_support::Database;

#[test]
fn migration_preserves_existing_snapshots_and_reapplies_after_a_later_version() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let case = db.seed_case();
    let first = Uuid::new_v4();
    let legacy = Uuid::new_v4();
    for (id, version) in [(first, 1_i64), (legacy, 7)] {
        db.client.execute("INSERT INTO documents(id,case_id,version,name,digest,vault,evidence) VALUES($1,$2,$3,'original.txt',$4,$5,$6)", &[&id,&case,&version, &vec![3_u8;32], &vec![7_u8;80], &serde_json::json!({"captured":[0,255,42]})]).unwrap();
    }
    db.client.execute("INSERT INTO audit_events VALUES(0,'2026-01-01T00:00:00.123456789Z','original','captured','unaltered',$1)", &[&vec![9_u8;32]]).unwrap();
    let before = db.snapshot();

    PostgresCaseDocumentStore::connect(&db.url).unwrap();

    assert_eq!(db.snapshot(), before);
    for (id, expected) in [(first, 1_i64), (legacy, 7)] {
        let row = db
            .client
            .query_one(
                "SELECT case_id,first_available_version FROM document_series WHERE id=$1",
                &[&id],
            )
            .unwrap();
        assert_eq!(row.get::<_, Uuid>(0), case);
        assert_eq!(row.get::<_, i64>(1), expected);
    }
    db.client.execute("INSERT INTO documents(id,case_id,version,name,digest,vault) VALUES($1,$2,8,'later.txt',$3,$4)", &[&legacy,&case,&vec![4_u8;32],&vec![8_u8;80]]).unwrap();
    let appended = db.snapshot();
    for _ in 0..3 {
        PostgresCaseDocumentStore::connect(&db.url).unwrap();
    }
    assert_eq!(db.snapshot(), appended);
    assert_eq!(
        db.client
            .query_one(
                "SELECT first_available_version FROM document_series WHERE id=$1",
                &[&legacy]
            )
            .unwrap()
            .get::<_, i64>(0),
        7
    );
}

#[test]
fn roots_and_snapshots_require_matching_first_version_case_and_contiguous_appends() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let case = db.seed_case();
    let foreign = db.seed_case();
    PostgresCaseDocumentStore::connect(&db.url).unwrap();
    let id = Uuid::new_v4();
    let mut empty = db.client.transaction().unwrap();
    empty
        .execute("INSERT INTO document_series VALUES($1,$2,7)", &[&id, &case])
        .unwrap();
    assert!(empty.commit().is_err(), "an empty root must not commit");
    let mut initial = db.client.transaction().unwrap();
    initial
        .execute("INSERT INTO document_series VALUES($1,$2,7)", &[&id, &case])
        .unwrap();
    initial
        .execute(
            "INSERT INTO documents VALUES($1,$2,7,'legacy.txt',$3,$4,NULL)",
            &[&id, &case, &vec![3_u8; 32], &vec![7_u8; 80]],
        )
        .unwrap();
    initial.commit().unwrap();
    for (candidate_case, version) in [(case, 6_i64), (case, 7), (case, 9), (foreign, 8)] {
        assert!(db
            .client
            .execute(
                "INSERT INTO documents VALUES($1,$2,$3,'bad.txt',$4,$5,NULL)",
                &[
                    &id,
                    &candidate_case,
                    &version,
                    &vec![3_u8; 32],
                    &vec![7_u8; 80]
                ]
            )
            .is_err());
    }
    db.client
        .execute(
            "INSERT INTO documents VALUES($1,$2,8,'next.txt',$3,$4,NULL)",
            &[&id, &case, &vec![4_u8; 32], &vec![8_u8; 80]],
        )
        .unwrap();
    for statement in [
        "UPDATE document_series SET first_available_version=8",
        "UPDATE document_series SET id=gen_random_uuid()",
        "UPDATE document_series SET case_id=gen_random_uuid()",
    ] {
        assert!(db.client.batch_execute(statement).is_err(), "{statement}");
    }
}

#[test]
fn a_restricted_runtime_role_can_append_without_update_privilege_on_roots() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let case = db.seed_case();
    let role = format!("versions_runtime_{}", Uuid::new_v4().simple());
    db.control
        .batch_execute(&format!(
            "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'"
        ))
        .unwrap();
    initialize_database(&db.url, &role).unwrap();
    let mut parsed = reqwest::Url::parse(&db.url).unwrap();
    parsed.set_username(&role).unwrap();
    parsed.set_password(Some("runtime-test-only")).unwrap();
    let mut runtime = Client::connect(parsed.as_str(), NoTls).unwrap();
    assert!(!runtime
        .query_one(
            "SELECT has_any_column_privilege(current_user,'document_series','UPDATE')",
            &[]
        )
        .unwrap()
        .get::<_, bool>(0));
    let id = Uuid::new_v4();
    let mut tx = runtime.transaction().unwrap();
    tx.execute("INSERT INTO document_series VALUES($1,$2,1)", &[&id, &case])
        .unwrap();
    tx.execute(
        "INSERT INTO documents VALUES($1,$2,1,'initial.txt',$3,$4,NULL)",
        &[&id, &case, &vec![3_u8; 32], &vec![7_u8; 80]],
    )
    .unwrap();
    tx.commit().unwrap();
    runtime
        .execute(
            "INSERT INTO documents VALUES($1,$2,2,'next.txt',$3,$4,NULL)",
            &[&id, &case, &vec![4_u8; 32], &vec![8_u8; 80]],
        )
        .unwrap();
    assert!(runtime.batch_execute("UPDATE document_series SET first_available_version=first_available_version WHERE false").is_err());
    drop(runtime);
    db.control
        .batch_execute(&format!(
            "DROP SCHEMA {} CASCADE; DROP ROLE {role}",
            db.schema
        ))
        .unwrap();
}
