#[path = "case_administration_support/mod.rs"]
mod case_administration_support;

use application::ApplicationError;
use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use postgres::error::SqlState;
use std::sync::Arc;

const TABLES: [&str; 4] = [
    "case_report_jobs",
    "case_report_snapshots",
    "case_report_artifacts",
    "case_report_notices",
];

#[test]
fn migration_creates_empty_report_tables_and_preserves_the_existing_case_inventory() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
    for table in TABLES {
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "migration invented report state in {table}");
        for operation in ["DELETE FROM", "TRUNCATE"] {
            let error = db
                .runtime()
                .batch_execute(&format!("{operation} {table}"))
                .unwrap_err();
            assert_eq!(error.code(), Some(&SqlState::INSUFFICIENT_PRIVILEGE));
        }
    }
    open(&db).unwrap();
}

#[test]
fn startup_rejects_report_table_shape_row_security_and_disabled_guards() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    for table in TABLES {
        for (damage, repair) in [
            (
                format!("ALTER TABLE {table} ADD COLUMN extra boolean"),
                format!("ALTER TABLE {table} DROP COLUMN extra"),
            ),
            (
                format!("ALTER TABLE {table} ENABLE ROW LEVEL SECURITY"),
                format!("ALTER TABLE {table} DISABLE ROW LEVEL SECURITY"),
            ),
            (
                format!("ALTER TABLE {table} DISABLE TRIGGER USER"),
                format!("ALTER TABLE {table} ENABLE TRIGGER USER"),
            ),
        ] {
            db.admin.batch_execute(&damage).unwrap();
            assert!(
                matches!(open(&db), Err(ApplicationError::InvalidConfiguration(_))),
                "accepted changed report schema: {damage}"
            );
            db.admin.batch_execute(&repair).unwrap();
            open(&db).unwrap();
        }
    }
}

#[test]
fn startup_rejects_a_report_check_replaced_with_true_under_its_original_name() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    for table in TABLES {
        let row = db
            .admin
            .query_one(
                "SELECT quote_ident(conname),pg_get_constraintdef(oid) FROM pg_constraint
                WHERE conrelid=$1::text::regclass AND contype='c' ORDER BY conname LIMIT 1",
                &[&table],
            )
            .unwrap();
        let name: String = row.get(0);
        let definition: String = row.get(1);
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE {table} DROP CONSTRAINT {name};
            ALTER TABLE {table} ADD CONSTRAINT {name} CHECK(true)"
            ))
            .unwrap();
        assert!(
            matches!(open(&db), Err(ApplicationError::InvalidConfiguration(_))),
            "accepted a weakened report constraint on {table}"
        );
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE {table} DROP CONSTRAINT {name};
            ALTER TABLE {table} ADD CONSTRAINT {name} {definition}"
            ))
            .unwrap();
        open(&db).unwrap();
    }
}

#[test]
fn runtime_rejects_public_delegated_or_unrestricted_report_authority() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    for table in TABLES {
        for (grant, revoke) in [
            (
                format!("GRANT SELECT ON {table} TO PUBLIC"),
                format!("REVOKE SELECT ON {table} FROM PUBLIC"),
            ),
            (
                format!("GRANT SELECT ON {table} TO {} WITH GRANT OPTION", db.role),
                format!("REVOKE GRANT OPTION FOR SELECT ON {table} FROM {}", db.role),
            ),
            (
                format!("GRANT UPDATE ON {table} TO {}", db.role),
                restore_column_updates(table, &db.role),
            ),
        ] {
            db.admin.batch_execute(&grant).unwrap();
            assert!(
                matches!(open(&db), Err(ApplicationError::InvalidConfiguration(_))),
                "accepted unsafe report permission: {grant}"
            );
            db.admin.batch_execute(&revoke).unwrap();
            open(&db)
                .unwrap_or_else(|error| panic!("permission repair failed after {grant}: {error}"));
        }
    }
}

fn restore_column_updates(table: &str, role: &str) -> String {
    let mut repair = format!("REVOKE UPDATE ON {table} FROM {role};");
    let columns = match table {
        "case_report_jobs" => "updated_at,state,failure,retry_at,lease_attempt,lease_token,lease_generation,lease_expires_at,attempts,case_ids",
        "case_report_notices" => "read_at",
        _ => return repair,
    };
    // Revoking a table privilege also removes its corresponding column grants.
    repair.push_str(&format!("GRANT UPDATE({columns}) ON {table} TO {role}"));
    repair
}

fn require_tables(db: &mut Fixture) {
    for table in TABLES {
        let present: bool = db
            .admin
            .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
            .unwrap()
            .get(0);
        assert!(present, "report migration did not create {table}");
    }
}

fn open(db: &Fixture) -> Result<PostgresCaseRepository, ApplicationError> {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
}
