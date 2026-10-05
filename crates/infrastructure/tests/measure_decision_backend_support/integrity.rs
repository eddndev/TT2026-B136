use super::*;
use application::{documents::StageSupportReadLimits, ApplicationError};

const TABLES: [&str; 4] = [
    "case_measure_operations",
    "case_measure_decisions",
    "case_measures",
    "case_measure_revisions",
];

fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn damage(db: &mut Fixture, statement: &str) {
    let mut tx = db.admin.transaction().unwrap();
    for table in TABLES {
        tx.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.batch_execute(statement).unwrap();
    for table in TABLES {
        tx.batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.commit().unwrap();
}

fn reject_reads_and_replay(
    storage: &PostgresMeasureDecisionStore,
    actor: &Principal,
    case: CaseId,
    command: &MeasureDecisionCommand,
) {
    assert!(MeasureDecisionReadStore::get(storage, actor, case, command.decision_id).is_err());
    assert!(
        MeasureDecisionReadStore::get_operation(storage, actor, case, command.operation_id)
            .is_err()
    );
    assert!(MeasureDecisionReadStore::list(
        storage,
        actor,
        case,
        MeasureDecisionReadQuery::default()
    )
    .is_err());
    assert!(MeasureDecisionStore::prepare(
        storage,
        actor,
        case,
        command,
        &StageSupportReadLimits::default()
    )
    .is_err());
}

#[test]
fn altered_commitments_missing_siblings_and_lost_roots_reject_on_open_connections() {
    for statement in [
        "UPDATE case_measure_decisions SET group_digest=decode(repeat('a1',32),'hex')",
        "UPDATE case_measure_revisions SET capture_digest=decode(repeat('b2',32),'hex')",
        "DELETE FROM case_measure_revisions WHERE measure_id=(SELECT measure_id FROM case_measure_revisions ORDER BY measure_id LIMIT 1)",
        "DELETE FROM case_measures WHERE id=(SELECT id FROM case_measures ORDER BY id LIMIT 1)",
        "DELETE FROM case_measure_decisions",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let seed = setup(&mut db);
        persist(&db, seed.actor.clone(), seed.command.clone());
        let storage = store(&db);
        damage(&mut db, statement);
        let before = snapshot(&mut db);
        reject_reads_and_replay(&storage, &seed.actor, db.case, &seed.command);
        assert!(open(&db).is_err(), "accepted damaged owner: {statement}");
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn orphan_zero_row_audit_blocks_false_empty_lists_and_global_fresh_identity_admission() {
    let Some(mut db) = Fixture::new() else { return };
    let old = setup(&mut db);
    let old_case = db.case;
    let command = no_change(&old.command);
    persist(&db, old.actor.clone(), command.clone());
    let other = setup(&mut db);
    let storage = store(&db);
    damage(
        &mut db,
        "DELETE FROM case_measure_decisions; DELETE FROM case_measure_operations",
    );
    let before = snapshot(&mut db);
    reject_reads_and_replay(&storage, &old.actor, old_case, &command);
    assert!(
        MeasureDecisionStore::prepare(
            storage.as_ref(),
            &other.actor,
            db.case,
            &other.command,
            &StageSupportReadLimits::default(),
        )
        .is_err(),
        "another case cannot certify global measure identity freshness after owner loss"
    );
    assert!(open(&db).is_err());
    assert_eq!(snapshot(&mut db), before);
}

mod clocks;
mod identities;
mod rollback;
