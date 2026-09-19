mod case_administration_support;
mod deadline_schema_support;

use case_administration_support::Fixture;
use deadline_schema_support::open;

fn tracking_columns(db: &mut Fixture) {
    for (name, data_type, generated) in [
        ("tracking_canonical", "bytea", ""),
        ("observations_canonical", "bytea", ""),
        ("tracking_administration_revision", "bigint", "s"),
        ("cause_event_sequence", "bigint", "s"),
    ] {
        let row = db.admin.query_opt(
            "SELECT atttypid::regtype::text,attnotnull,attgenerated::text FROM pg_attribute WHERE attrelid='case_deadline_revisions'::regclass AND attname=$1 AND NOT attisdropped",
            &[&name],
        ).unwrap().unwrap_or_else(|| panic!("missing column {name}"));
        assert_eq!(row.get::<_, String>(0), data_type);
        assert!(!row.get::<_, bool>(1));
        assert_eq!(row.get::<_, String>(2), generated);
    }
}

#[test]
fn migration_keeps_tracking_separate_from_historical_calculation() {
    let Some(mut db) = Fixture::new() else { return };
    tracking_columns(&mut db);
    for name in ["recorded_by", "recorded_by_email"] {
        let required: bool = db.admin.query_one(
            "SELECT attnotnull FROM pg_attribute WHERE attrelid='case_deadline_revisions'::regclass AND attname=$1",
            &[&name],
        ).unwrap().get(0);
        assert!(!required, "technical author cannot use a fictitious {name}");
    }
    for name in [
        "observed_administration_revision",
        "observed_administration_canonical",
        "source_head_revision",
        "calendar_head_revision",
        "due_at_seconds",
    ] {
        let exists: bool = db.admin.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='case_deadline_revisions'::regclass AND attname=$1 AND NOT attisdropped)",
            &[&name],
        ).unwrap().get(0);
        assert!(exists, "historical column removed: {name}");
    }
    open(&db).unwrap();
    let before = db.snapshot();
    db.migrate();
    tracking_columns(&mut db);
    open(&db).unwrap();
    assert_eq!(db.snapshot(), before);
}

#[test]
fn startup_requires_tracking_columns_with_the_exact_nullability_and_projections() {
    for alteration in [
        "ALTER TABLE case_deadline_revisions ALTER COLUMN tracking_canonical SET NOT NULL",
        "ALTER TABLE case_deadline_revisions ALTER COLUMN observations_canonical SET NOT NULL",
        "ALTER TABLE case_deadline_revisions ALTER COLUMN tracking_administration_revision DROP EXPRESSION",
        "ALTER TABLE case_deadline_revisions ALTER COLUMN cause_event_sequence DROP EXPRESSION",
        "ALTER TABLE case_deadline_revisions ALTER COLUMN recorded_by SET NOT NULL",
        "ALTER TABLE case_deadline_revisions ALTER COLUMN recorded_by_email SET NOT NULL",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        tracking_columns(&mut db);
        open(&db).unwrap();
        db.admin.batch_execute(alteration).unwrap();
        assert!(open(&db).is_err(), "accepted {alteration}");
    }
}

#[test]
fn startup_requires_exact_tracking_foreign_keys_and_commitment_checks() {
    for name in [
        "deadline_tracking_administration",
        "deadline_cause_event",
        "deadline_tracking_shape",
        "deadline_tracking_commitments",
        "deadline_recorded_author",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        tracking_columns(&mut db);
        open(&db).unwrap();
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE case_deadline_revisions DROP CONSTRAINT {name}"
            ))
            .unwrap();
        assert!(open(&db).is_err(), "accepted missing {name}");
    }
}
