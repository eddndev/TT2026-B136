mod case_administration_support;

use case_administration_support::Fixture;

#[test]
fn migration_adds_empty_calendar_history_without_inventing_declared_calendars() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["judicial_calendars", "judicial_calendar_revisions"] {
        let exists: bool = db
            .admin
            .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
            .unwrap()
            .get(0);
        assert!(exists, "calendar migration must install {table}");
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "migration must not invent declared calendars");
    }
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
}

#[test]
fn runtime_can_append_calendars_but_cannot_rewrite_their_history() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["judicial_calendars", "judicial_calendar_revisions"] {
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
        "judicial_calendar_values(bytea)",
        "judicial_calendar_submission(bytea)",
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
fn calendar_history_rejects_even_privileged_empty_table_mutations() {
    let Some(mut db) = Fixture::new() else { return };
    for sql in [
        "UPDATE judicial_calendars SET id=id",
        "DELETE FROM judicial_calendar_revisions",
        "TRUNCATE judicial_calendars CASCADE",
    ] {
        assert_eq!(
            db.admin.batch_execute(sql).unwrap_err().code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
