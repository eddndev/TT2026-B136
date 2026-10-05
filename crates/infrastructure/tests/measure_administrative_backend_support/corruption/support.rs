use super::*;

pub(super) const ADMINISTRATIVE_TABLES: [&str; 4] = [
    "case_measure_administrations",
    "case_measure_operations",
    "case_measure_revisions",
    "audit_events",
];

pub(super) fn existing_service(
    db: &Fixture,
    actor: Principal,
    storage: Arc<PostgresMeasureAdministrativeStore>,
) -> MeasureAdministrativeService {
    MeasureAdministrativeService::new(
        storage,
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(FormatCheck(Some(Box::new(|| {
            panic!("a corrupt replay must never reach documentary admission")
        })))),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub(super) fn damage(db: &mut Fixture, tables: &[&str], statement: &str) {
    let mut tx = db.admin.transaction().unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.batch_execute(statement).unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.commit().unwrap();
}

pub(super) fn reject_original(
    db: &mut Fixture,
    storage: &PostgresMeasureAdministrativeStore,
    workflow: &MeasureAdministrativeService,
    actor: &Principal,
    original: &MeasureAdministrativeStoredOperation,
) {
    let before = snapshot(db);
    let command = &original.capture.review.command;
    assert!(storage
        .prepare(actor, db.case, command, &StageSupportReadLimits::standard())
        .is_err());
    assert!(workflow
        .prepare("session", db.case, command.clone())
        .is_err());
    assert!(workflow
        .submit(
            "session",
            db.case,
            command.clone(),
            confirmation(&original.capture.review)
        )
        .is_err());
    assert!(open(db).is_err());
    assert_eq!(snapshot(db), before);
}

fn open(db: &Fixture) -> Result<PostgresMeasureAdministrativeStore, ApplicationError> {
    PostgresMeasureAdministrativeStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
