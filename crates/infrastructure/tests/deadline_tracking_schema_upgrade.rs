mod case_administration_support;
mod deadline_tracking_upgrade_support;

use application::deadlines::*;
use deadline_tracking_upgrade_support::*;
use domain::clock::{Clock, OffsetDateTime};
use infrastructure::{PostgresCaseRepository, PostgresDeadlineStore, RingSha256Hasher};
use std::sync::Arc;

struct FixedClock(OffsetDateTime);
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}

fn open_inventory(db: &Fixture) {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
}

fn parser_oid(db: &mut Fixture) -> i64 {
    db.admin
        .query_one(
            "SELECT 'deadline_submission(bytea)'::regprocedure::oid::bigint",
            &[],
        )
        .unwrap()
        .get(0)
}

fn assert_generated_parser_dependency(db: &mut Fixture, oid: i64) {
    let valid: bool = db
        .admin
        .query_one(
            "SELECT EXISTS(
        SELECT 1 FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum
        JOIN pg_depend dep ON dep.classid='pg_attrdef'::regclass AND dep.objid=d.oid
        WHERE d.adrelid='case_deadline_revisions'::regclass AND a.attname='submission_view'
        AND dep.refclassid='pg_proc'::regclass AND dep.refobjid=$1::bigint::oid)",
            &[&oid],
        )
        .unwrap()
        .get(0);
    assert!(
        valid,
        "stored projection must still reference the original parser OID"
    );
}

#[test]
fn upgrading_an_actual_0017_schema_adds_nullable_tracking_without_changing_existing_case_data() {
    let Some(mut db) = old_deadlines() else {
        return;
    };
    assert_ne!(db.schema, "public");
    let before = snapshot(&mut db, false);
    assert!(before["revisions"].is_null());
    db.migrate();
    assert_columns(&mut db);
    assert_eq!(snapshot(&mut db, true), before);
    open_inventory(&db);
    db.migrate();
    assert_columns(&mut db);
    assert_eq!(snapshot(&mut db, true), before);
    open_inventory(&db);
}

#[test]
fn actual_v1_rows_receipts_and_dependency_events_survive_upgrade_and_repeated_migration() {
    let Some(mut db) = old_deadlines() else {
        return;
    };
    let history = seed_history(&mut db);
    let before = snapshot(&mut db, false);
    assert_eq!(before["revisions"].as_array().unwrap().len(), 4);
    assert_eq!(before["deadlines"].as_array().unwrap().len(), 2);
    assert_eq!(before["profile_revisions"].as_array().unwrap().len(), 1);
    assert_eq!(before["events"].as_array().unwrap().len(), 1);
    let original_oid = parser_oid(&mut db);
    assert_generated_parser_dependency(&mut db, original_oid);
    for _ in 0..3 {
        db.migrate();
        assert_columns(&mut db);
        assert_eq!(parser_oid(&mut db), original_oid);
        assert_generated_parser_dependency(&mut db, original_oid);
        assert_eq!(snapshot(&mut db, true), before);
        open_inventory(&db);
        assert_eq!(
            snapshot(&mut db, true),
            before,
            "inventory must not rewrite history or audit"
        );
        let projection_matches: bool = db
            .admin
            .query_one(
                "SELECT bool_and(
            submission_view=deadline_submission(submission_canonical)
            AND substring(submission_canonical FROM 1 FOR 5)=convert_to('DLTX1','UTF8')
            AND substring(review_canonical FROM 1 FOR 5)=convert_to('DLRV1','UTF8')
            AND substring(capture_canonical FROM 1 FOR 5)=convert_to('DLST1','UTF8'))
            FROM case_deadline_revisions",
                &[],
            )
            .unwrap()
            .get(0);
        assert!(projection_matches);
    }
    let store = PostgresDeadlineStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
    .unwrap();
    for expected in history {
        let actual = store
            .get(
                db.owner,
                db.case,
                expected.id,
                Some(expected.revision),
                db.at,
            )
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.receipt.version, DeadlineReceiptVersion::Legacy);
        assert!(actual.tracking.is_none());
    }
}
