mod case_administration_support;

use case_administration_support::Fixture;

#[test]
fn migration_adds_empty_fact_history_without_inventing_declared_facts() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["case_procedural_facts", "case_procedural_fact_revisions"] {
        let exists: bool = db
            .admin
            .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
            .unwrap()
            .get(0);
        assert!(exists, "fact migration must install {table}");
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "migration must not invent declared facts");
    }
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
}

#[test]
fn runtime_can_append_facts_but_cannot_rewrite_their_history() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["case_procedural_facts", "case_procedural_fact_revisions"] {
        let row = db
            .admin
            .query_one(
                "SELECT has_table_privilege($1,$2,'SELECT'),
             has_table_privilege($1,$2,'INSERT'),
             has_table_privilege($1,$2,'UPDATE,DELETE,TRUNCATE,TRIGGER')",
                &[&db.role, &table],
            )
            .unwrap();
        assert!(row.get::<_, bool>(0));
        assert!(row.get::<_, bool>(1));
        assert!(!row.get::<_, bool>(2));
    }
    for function in [
        "procedural_fact_values(text,bytea)",
        "procedural_fact_submission(bytea)",
        "procedural_fact_sources(bytea)",
    ] {
        let allowed: bool = db
            .admin
            .query_one(
                "SELECT has_function_privilege($1,$2,'EXECUTE')",
                &[&db.role, &function],
            )
            .unwrap()
            .get(0);
        assert!(allowed);
    }
}

#[test]
fn fact_history_rejects_even_privileged_empty_table_mutations() {
    let Some(mut db) = Fixture::new() else { return };
    for sql in [
        "UPDATE case_procedural_facts SET id=id",
        "DELETE FROM case_procedural_fact_revisions",
        "TRUNCATE case_procedural_facts CASCADE",
    ] {
        assert_eq!(
            db.admin.batch_execute(sql).unwrap_err().code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
