use crate::password_reset_backend_support::*;
use crate::support::*;

fn check(url: &str) -> std::process::Output {
    invoke(Some(url), &args(&["--json", "database", "check"]))
}

#[test]
fn database_check_accepts_a_read_only_runtime_without_any_database_or_catalog_change() {
    let mut db = fixture();
    let user = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, user, 1);
    let before = state(&mut db);
    let checked = success(check(&read_only_url(&db.runtime_url)));
    assert_eq!(checked, serde_json::json!({"validated":true}));
    unchanged(&mut db, &before);
    assert!(repository.inspect(digest(1)).unwrap().is_some());
}

#[test]
fn database_check_rejects_an_administrator_and_does_not_upgrade_an_old_schema() {
    let mut current = fixture();
    let before = state(&mut current);
    rejected(check(&current.admin_url));
    unchanged(&mut current, &before);
    let mut old = Fixture::old().expect("database CLI tests require CASE_TEST_DATABASE_URL");
    let before = state(&mut old);
    rejected(check(&read_only_url(&old.runtime_url)));
    unchanged(&mut old, &before);
    let reset_table: Option<String> = old
        .admin
        .query_one(
            "SELECT to_regclass('password_reset_capabilities')::text",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(reset_table.is_none(), "database check applied a migration");
}

#[test]
fn database_check_rejects_and_preserves_bad_catalog_authority_and_business_inventory() {
    for condition in ["catalog", "authority", "inventory"] {
        let mut db = fixture();
        match condition {
            "catalog" => db.admin.batch_execute(
                "ALTER FUNCTION password_reset_consume(uuid,bytea,uuid,bigint,text) SET search_path=public",
            ).unwrap(),
            "authority" => db.admin.batch_execute(&format!(
                "GRANT SELECT ON password_reset_capabilities TO {}", db.role,
            )).unwrap(),
            "inventory" => {
                db.admin.execute(
                    "INSERT INTO case_administration_revisions(
                     case_id,revision,title,reference,administrative_status,values_digest,
                     changed_at,changed_by,changed_by_email)
                     VALUES($1,1,'Baseline','REF-OLD','active',
                     sha256(case_administration_bytes('active','Baseline','REF-OLD',NULL,NULL,NULL,NULL,NULL,NULL,NULL)),
                     '2025-01-01T00:00:00Z',$2,' invalid ')",
                    &[&db.case.as_uuid(), &db.owner.as_uuid()],
                ).unwrap();
            }
            _ => unreachable!(),
        }
        let before = state(&mut db);
        rejected(check(&read_only_url(&db.runtime_url)));
        unchanged(&mut db, &before);
    }
}
